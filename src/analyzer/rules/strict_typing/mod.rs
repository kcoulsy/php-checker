pub use crate::analyzer::rules::{DiagnosticRule, helpers};

pub mod missing_argument;
pub mod missing_return;
pub mod strict_types;
pub mod type_mismatch;

pub use missing_argument::MissingArgumentRule;
pub use missing_return::MissingReturnRule;
pub use strict_types::StrictTypesRule;
pub use type_mismatch::TypeMismatchRule;
