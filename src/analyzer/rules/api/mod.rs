pub use crate::analyzer::rules::{DiagnosticRule, helpers};

pub mod deprecated_api;
pub mod invalid_this;

pub use deprecated_api::DeprecatedApiRule;
pub use invalid_this::InvalidThisRule;
