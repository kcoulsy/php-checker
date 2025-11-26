pub use crate::analyzer::rules::{DiagnosticRule, helpers};

pub mod duplicate_switch_case;
pub mod fallthrough;
pub mod impossible_comparison;
pub mod redundant_condition;
pub mod unreachable;
pub mod unreachable_statement;

pub use duplicate_switch_case::DuplicateSwitchCaseRule;
pub use fallthrough::FallthroughRule;
pub use impossible_comparison::ImpossibleComparisonRule;
pub use redundant_condition::RedundantConditionRule;
pub use unreachable::UnreachableCodeRule;
pub use unreachable_statement::UnreachableStatementRule;
