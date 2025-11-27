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
        let value = value.trim();

        // Find where the variable name starts (marked by $)
        let dollar_pos = value.find('$')?;

        // Type is everything before the $, trimmed
        let type_str = value[..dollar_pos].trim();
        let var_part = &value[dollar_pos..];

        let type_expr = Self::parse_type_expression(type_str)?;

        // Extract variable name (first token after $)
        let parts: Vec<&str> = var_part.splitn(2, char::is_whitespace).collect();
        let var_name = parts[0].trim_start_matches('$');

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
        let value = value.trim();

        // Find where the variable name starts (marked by $)
        // The type is everything before that, or the whole string if no variable name
        let (type_str, rest) = if let Some(dollar_pos) = value.find('$') {
            // Type is everything before the $, trimmed
            let type_part = value[..dollar_pos].trim();
            let var_part = &value[dollar_pos..];
            (type_part, Some(var_part))
        } else {
            // No variable name, entire string is the type
            (value, None)
        };

        let type_expr = Self::parse_type_expression(type_str)?;

        let name = rest.and_then(|s| {
            let parts: Vec<&str> = s.splitn(2, char::is_whitespace).collect();
            if !parts.is_empty() && parts[0].starts_with('$') {
                Some(parts[0].trim_start_matches('$').to_string())
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

        // Handle shaped arrays: array{name: string, age: int}
        if type_str.starts_with("array{") && type_str.ends_with('}') {
            let fields_str = &type_str[6..type_str.len() - 1]; // Remove "array{" and "}"
            let fields = Self::parse_shaped_array_fields(fields_str)?;
            return Some(TypeExpression::ShapedArray(fields));
        }

        // Handle generic types: array<Type>, array<Key, Value>
        if let Some((base, params_str)) = Self::split_generic(type_str) {
            // Split parameters respecting nested braces and angle brackets
            let param_strs = Self::split_params(params_str);
            let params: Option<Vec<_>> = param_strs
                .iter()
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

    /// Parse shaped array fields: "name: string, age: int" -> [("name", Simple("string")), ("age", Simple("int"))]
    fn parse_shaped_array_fields(fields_str: &str) -> Option<Vec<(String, TypeExpression)>> {
        let field_strs = Self::split_params(fields_str);
        let mut fields = Vec::new();

        for field_str in field_strs {
            // Split on colon: "name: string" -> ["name", "string"]
            let parts: Vec<&str> = field_str.splitn(2, ':').collect();
            if parts.len() != 2 {
                return None;
            }

            let field_name = parts[0].trim().to_string();
            let type_str = parts[1].trim();
            let type_expr = Self::parse_type_expression(type_str)?;

            fields.push((field_name, type_expr));
        }

        Some(fields)
    }

    /// Split comma-separated parameters while respecting nesting
    /// Example: "int, array{name: string, age: int}" -> ["int", "array{name: string, age: int}"]
    fn split_params(params_str: &str) -> Vec<String> {
        let mut result = Vec::new();
        let mut current = String::new();
        let mut depth = 0; // Track nesting depth for {} and <>

        for ch in params_str.chars() {
            match ch {
                '{' | '<' | '(' | '[' => {
                    depth += 1;
                    current.push(ch);
                }
                '}' | '>' | ')' | ']' => {
                    depth -= 1;
                    current.push(ch);
                }
                ',' if depth == 0 => {
                    // Top-level comma - split here
                    if !current.trim().is_empty() {
                        result.push(current.trim().to_string());
                    }
                    current.clear();
                }
                _ => {
                    current.push(ch);
                }
            }
        }

        // Don't forget the last parameter
        if !current.trim().is_empty() {
            result.push(current.trim().to_string());
        }

        result
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

    #[test]
    fn test_split_params_with_nesting() {
        // Simple case
        let params = PhpDocParser::split_params("int, string");
        assert_eq!(params, vec!["int", "string"]);

        // Nested braces
        let params = PhpDocParser::split_params("int, array{name: string, age: int}");
        assert_eq!(params, vec!["int", "array{name: string, age: int}"]);

        // Nested angle brackets
        let params = PhpDocParser::split_params("string, array<int, User>");
        assert_eq!(params, vec!["string", "array<int, User>"]);

        // Complex nesting
        let params = PhpDocParser::split_params("int, array<string, array{id: int, data: string}>");
        assert_eq!(params, vec!["int", "array<string, array{id: int, data: string}>"]);
    }

    #[test]
    fn test_parse_var_tag_with_generic_array() {
        let comment = r#"/**
         * @var array<string, int>
         */"#;

        let doc = PhpDocParser::parse(comment).unwrap();
        assert!(doc.var_tag.is_some());
        let var_tag = doc.var_tag.unwrap();
        eprintln!("DEBUG: Parsed type_expr = {:?}", var_tag.type_expr);
        match var_tag.type_expr {
            TypeExpression::Generic { base, params } => {
                assert_eq!(base, "array");
                assert_eq!(params.len(), 2);
                assert!(matches!(params[0], TypeExpression::Simple(ref s) if s == "string"));
                assert!(matches!(params[1], TypeExpression::Simple(ref s) if s == "int"));
            }
            _ => panic!("Expected Generic type for array<string, int>, got: {:?}", var_tag.type_expr),
        }
    }
}
