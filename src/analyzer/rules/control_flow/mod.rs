pub use crate::analyzer::rules::{DiagnosticRule, helpers};

pub mod impossible_comparison;
pub mod redundant_condition;
pub mod unreachable;

pub use impossible_comparison::ImpossibleComparisonRule;
pub use redundant_condition::RedundantConditionRule;
pub use unreachable::UnreachableCodeRule;
