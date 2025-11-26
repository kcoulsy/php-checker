pub use crate::analyzer::rules::{DiagnosticRule, helpers};

pub mod array_key_not_defined;
pub mod duplicate_declaration;
pub mod undefined_variable;

pub use array_key_not_defined::ArrayKeyNotDefinedRule;
pub use duplicate_declaration::DuplicateDeclarationRule;
pub use undefined_variable::UndefinedVariableRule;
