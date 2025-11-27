/// Represents a type expression from PHPDoc
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypeExpression {
    /// Simple type: int, string, User, etc.
    Simple(String),

    /// Array type: User[], int[]
    Array(Box<TypeExpression>),

    /// Generic type: array<string, int>, Collection<User>
    Generic {
        base: String,
        params: Vec<TypeExpression>,
    },

    /// Union type: int|string|null
    Union(Vec<TypeExpression>),

    /// Nullable type: ?string
    Nullable(Box<TypeExpression>),

    /// Mixed type
    Mixed,

    /// Void type
    Void,

    /// Never type
    Never,
}

impl TypeExpression {
    /// Check if this type is nullable
    pub fn is_nullable(&self) -> bool {
        matches!(self, TypeExpression::Nullable(_))
            || matches!(self, TypeExpression::Union(types) if types.iter().any(|t| matches!(t, TypeExpression::Simple(s) if s == "null")))
    }

    /// Get the inner type if this is nullable
    pub fn unwrap_nullable(&self) -> &TypeExpression {
        match self {
            TypeExpression::Nullable(inner) => inner.as_ref(),
            _ => self,
        }
    }

    /// Check if this type expression contains a specific simple type
    pub fn contains_type(&self, type_name: &str) -> bool {
        match self {
            TypeExpression::Simple(s) => s == type_name,
            TypeExpression::Array(inner) => inner.contains_type(type_name),
            TypeExpression::Generic { params, .. } => {
                params.iter().any(|p| p.contains_type(type_name))
            }
            TypeExpression::Union(types) => types.iter().any(|t| t.contains_type(type_name)),
            TypeExpression::Nullable(inner) => inner.contains_type(type_name),
            _ => false,
        }
    }
}

/// @param tag
#[derive(Debug, Clone)]
pub struct ParamTag {
    pub name: String,
    pub type_expr: TypeExpression,
}

/// @return tag
#[derive(Debug, Clone)]
pub struct ReturnTag {
    pub type_expr: TypeExpression,
}

/// @var tag
#[derive(Debug, Clone)]
pub struct VarTag {
    pub name: Option<String>,
    pub type_expr: TypeExpression,
}

/// @throws tag
#[derive(Debug, Clone)]
pub struct ThrowsTag {
    pub exception_type: String,
    pub description: Option<String>,
}

/// @property tag
#[derive(Debug, Clone)]
pub struct PropertyTag {
    pub name: String,
    pub type_expr: TypeExpression,
    pub readonly: bool,
    pub writeonly: bool,
}

/// @method tag
#[derive(Debug, Clone)]
pub struct MethodTag {
    pub name: String,
    pub params: Vec<ParamTag>,
    pub return_type: Option<TypeExpression>,
    pub is_static: bool,
}
