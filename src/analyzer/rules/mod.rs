use super::project::ProjectContext;
use crate::analyzer::parser;

pub mod array_key_not_defined;
pub mod deprecated_api;
pub mod duplicate_declaration;
pub mod hard_coded_credentials;
pub mod helpers;
pub mod impossible_comparison;
pub mod include_user_input;
pub mod invalid_this;
pub mod missing_argument;
pub mod missing_return;
pub mod mutating_literal;
pub mod psr4;
pub mod redundant_condition;
pub mod strict_types;
pub mod type_mismatch;
pub mod undefined_variable;
pub mod unreachable;
pub mod unused_use;
pub mod unused_variable;

pub use array_key_not_defined::ArrayKeyNotDefinedRule;
pub use deprecated_api::DeprecatedApiRule;
pub use duplicate_declaration::DuplicateDeclarationRule;
pub use hard_coded_credentials::HardCodedCredentialsRule;
pub use impossible_comparison::ImpossibleComparisonRule;
pub use include_user_input::IncludeUserInputRule;
pub use invalid_this::InvalidThisRule;
pub use missing_argument::MissingArgumentRule;
pub use missing_return::MissingReturnRule;
pub use mutating_literal::MutatingLiteralRule;
pub use redundant_condition::RedundantConditionRule;
pub use strict_types::StrictTypesRule;
pub use type_mismatch::TypeMismatchRule;
pub use undefined_variable::UndefinedVariableRule;
pub use unreachable::UnreachableCodeRule;
pub use unused_use::UnusedUseRule;
pub use unused_variable::UnusedVariableRule;

pub trait DiagnosticRule {
    fn name(&self) -> &str;
    fn run(
        &self,
        parsed: &parser::ParsedSource,
        context: &ProjectContext,
    ) -> Vec<super::Diagnostic>;
}
