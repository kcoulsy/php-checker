pub use crate::analyzer::rules::{DiagnosticRule, helpers};

pub mod consistent_return;
pub mod force_return_type;
pub mod missing_argument;
pub mod missing_return;
pub mod phpdoc_param_check;
pub mod phpdoc_var_check;
pub mod strict_types;
pub mod type_mismatch;

pub use consistent_return::ConsistentReturnRule;
pub use force_return_type::ForceReturnTypeRule;
pub use missing_argument::MissingArgumentRule;
pub use missing_return::MissingReturnRule;
pub use phpdoc_param_check::PhpDocParamCheckRule;
pub use phpdoc_var_check::PhpDocVarCheckRule;
pub use strict_types::StrictTypesRule;
pub use type_mismatch::TypeMismatchRule;
