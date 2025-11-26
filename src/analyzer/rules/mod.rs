use super::project::ProjectContext;
use crate::analyzer::parser;

pub mod array_key_not_defined;
pub mod duplicate_declaration;
pub mod helpers;
pub mod impossible_comparison;
pub mod missing_argument;
pub mod missing_return;
pub mod redundant_condition;
pub mod type_mismatch;
pub mod undefined_variable;
pub mod unreachable;
pub mod unused_variable;

pub use array_key_not_defined::ArrayKeyNotDefinedRule;
pub use duplicate_declaration::DuplicateDeclarationRule;
pub use impossible_comparison::ImpossibleComparisonRule;
pub use missing_argument::MissingArgumentRule;
pub use missing_return::MissingReturnRule;
pub use redundant_condition::RedundantConditionRule;
pub use type_mismatch::TypeMismatchRule;
pub use undefined_variable::UndefinedVariableRule;
pub use unreachable::UnreachableCodeRule;
pub use unused_variable::UnusedVariableRule;

pub trait DiagnosticRule {
    fn name(&self) -> &str;
    fn run(
        &self,
        parsed: &parser::ParsedSource,
        context: &ProjectContext,
    ) -> Vec<super::Diagnostic>;
}
