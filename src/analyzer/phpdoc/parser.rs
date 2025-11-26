use super::types::*;

/// Represents a parsed PHPDoc comment
#[derive(Debug, Clone, Default)]
pub struct PhpDocComment {
    pub params: Vec<ParamTag>,
    pub return_tag: Option<ReturnTag>,
    pub var_tag: Option<VarTag>,
    pub throws: Vec<ThrowsTag>,
    pub properties: Vec<PropertyTag>,
    pub methods: Vec<MethodTag>,
}

pub struct PhpDocParser;

impl PhpDocParser {
    /// Parse a PHPDoc comment string
    pub fn parse(comment: &str) -> Option<PhpDocComment> {
        // Check if this is a valid PHPDoc comment
        if !comment.trim_start().starts_with("/**") {
            return None;
        }

        let mut doc = PhpDocComment::default();

        // Extract lines from the comment
        let lines = Self::extract_lines(comment);

        for line in lines {
            let line = line.trim();

            // Skip empty lines
            if line.is_empty() {
                continue;
            }

            // Parse tags
            if let Some(tag_content) = line.strip_prefix('@') {
                Self::parse_tag(tag_content, &mut doc);
            }
        }

        Some(doc)
    }

    /// Extract clean lines from PHPDoc comment
    fn extract_lines(comment: &str) -> Vec<String> {
        comment
            .lines()
            .map(|line| {
                line.trim()
                    .trim_start_matches("/**")
                    .trim_start_matches('*')
                    .trim_end_matches("*/")
                    .trim()
                    .to_string()
            })
            .filter(|line| !line.is_empty())
            .collect()
    }

    /// Parse a single tag line
    fn parse_tag(tag_content: &str, doc: &mut PhpDocComment) {
        let parts: Vec<&str> = tag_content.splitn(2, char::is_whitespace).collect();
        if parts.is_empty() {
            return;
        }

        let tag_name = parts[0];
        let tag_value = parts.get(1).unwrap_or(&"").trim();

        match tag_name {
            "param" | "phpstan-param" => {
                if let Some(param) = Self::parse_param_tag(tag_value) {
                    doc.params.push(param);
                }
            }
            "return" | "phpstan-return" => {
                if let Some(return_tag) = Self::parse_return_tag(tag_value) {
                    doc.return_tag = Some(return_tag);
                }
            }
            "var" | "phpstan-var" => {
                if let Some(var_tag) = Self::parse_var_tag(tag_value) {
                    doc.var_tag = Some(var_tag);
                }
            }
            "throws" => {
                if let Some(throws_tag) = Self::parse_throws_tag(tag_value) {
                    doc.throws.push(throws_tag);
                }
            }
            _ => {
                // Ignore other tags for now
            }
        }
    }

    /// Parse @param tag
    /// Format: @param Type $name [description]
    fn parse_param_tag(value: &str) -> Option<ParamTag> {
        let parts: Vec<&str> = value.splitn(3, char::is_whitespace).collect();
        if parts.len() < 2 {
            return None;
        }

        let type_str = parts[0];
        let var_name = parts[1].trim_start_matches('$');

        let type_expr = Self::parse_type_expression(type_str)?;

        Some(ParamTag {
            name: var_name.to_string(),
            type_expr,
        })
    }

    /// Parse @return tag
    /// Format: @return Type [description]
    fn parse_return_tag(value: &str) -> Option<ReturnTag> {
        let parts: Vec<&str> = value.splitn(2, char::is_whitespace).collect();
        if parts.is_empty() {
            return None;
        }

        let type_str = parts[0];
        let type_expr = Self::parse_type_expression(type_str)?;

        Some(ReturnTag { type_expr })
    }

    /// Parse @var tag
    /// Format: @var Type [$name] [description]
    fn parse_var_tag(value: &str) -> Option<VarTag> {
        let parts: Vec<&str> = value.splitn(3, char::is_whitespace).collect();
        if parts.is_empty() {
            return None;
        }

        let type_str = parts[0];
        let type_expr = Self::parse_type_expression(type_str)?;

        let name = parts.get(1).and_then(|s| {
            if s.starts_with('$') {
                Some(s.trim_start_matches('$').to_string())
            } else {
                None
            }
        });

        Some(VarTag { name, type_expr })
    }

    /// Parse @throws tag
    /// Format: @throws ExceptionType [description]
    fn parse_throws_tag(value: &str) -> Option<ThrowsTag> {
        let parts: Vec<&str> = value.splitn(2, char::is_whitespace).collect();
        if parts.is_empty() {
            return None;
        }

        let exception_type = parts[0].to_string();
        let description = parts.get(1).map(|s| s.to_string());

        Some(ThrowsTag {
            exception_type,
            description,
        })
    }

    /// Parse a type expression
    /// Supports: int, string, int[], array<string, int>, int|string, ?int, etc.
    pub fn parse_type_expression(type_str: &str) -> Option<TypeExpression> {
        let type_str = type_str.trim();

        // Handle nullable types: ?Type
        if let Some(inner) = type_str.strip_prefix('?') {
            let inner_expr = Self::parse_type_expression(inner)?;
            return Some(TypeExpression::Nullable(Box::new(inner_expr)));
        }

        // Handle union types: Type1|Type2|Type3
        if type_str.contains('|') {
            let types: Option<Vec<_>> = type_str
                .split('|')
                .map(|t| Self::parse_type_expression(t.trim()))
                .collect();
            return types.map(TypeExpression::Union);
        }

        // Handle array shorthand: Type[]
        if let Some(base) = type_str.strip_suffix("[]") {
            let inner_expr = Self::parse_type_expression(base)?;
            return Some(TypeExpression::Array(Box::new(inner_expr)));
        }

        // Handle generic types: array<Type>, array<Key, Value>
        if let Some((base, params_str)) = Self::split_generic(type_str) {
            let params: Option<Vec<_>> = params_str
                .split(',')
                .map(|p| Self::parse_type_expression(p.trim()))
                .collect();

            if let Some(params) = params {
                return Some(TypeExpression::Generic {
                    base: base.to_string(),
                    params,
                });
            }
        }

        // Special types
        match type_str {
            "mixed" => Some(TypeExpression::Mixed),
            "void" => Some(TypeExpression::Void),
            "never" => Some(TypeExpression::Never),
            _ => Some(TypeExpression::Simple(type_str.to_string())),
        }
    }

    /// Split a generic type into base and parameters
    /// Example: "array<string, int>" -> ("array", "string, int")
    fn split_generic(type_str: &str) -> Option<(&str, &str)> {
        let start = type_str.find('<')?;
        let end = type_str.rfind('>')?;

        if end <= start {
            return None;
        }

        let base = &type_str[..start];
        let params = &type_str[start + 1..end];

        Some((base, params))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_type() {
        let expr = PhpDocParser::parse_type_expression("int").unwrap();
        assert!(matches!(expr, TypeExpression::Simple(s) if s == "int"));
    }

    #[test]
    fn test_parse_nullable_type() {
        let expr = PhpDocParser::parse_type_expression("?string").unwrap();
        assert!(matches!(expr, TypeExpression::Nullable(_)));
    }

    #[test]
    fn test_parse_array_type() {
        let expr = PhpDocParser::parse_type_expression("int[]").unwrap();
        assert!(matches!(expr, TypeExpression::Array(_)));
    }

    #[test]
    fn test_parse_union_type() {
        let expr = PhpDocParser::parse_type_expression("int|string").unwrap();
        match expr {
            TypeExpression::Union(types) => assert_eq!(types.len(), 2),
            _ => panic!("Expected union type"),
        }
    }

    #[test]
    fn test_parse_generic_type() {
        let expr = PhpDocParser::parse_type_expression("array<string, int>").unwrap();
        match expr {
            TypeExpression::Generic { base, params } => {
                assert_eq!(base, "array");
                assert_eq!(params.len(), 2);
            }
            _ => panic!("Expected generic type"),
        }
    }

    #[test]
    fn test_parse_param_tag() {
        let param = PhpDocParser::parse_param_tag("int $value Some description").unwrap();
        assert_eq!(param.name, "value");
        assert!(matches!(param.type_expr, TypeExpression::Simple(s) if s == "int"));
    }

    #[test]
    fn test_parse_phpdoc_comment() {
        let comment = r#"/**
         * @param int $id
         * @param string $name
         * @return bool
         */"#;

        let doc = PhpDocParser::parse(comment).unwrap();
        assert_eq!(doc.params.len(), 2);
        assert!(doc.return_tag.is_some());
    }
}
