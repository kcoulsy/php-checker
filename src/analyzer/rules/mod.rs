use super::project::ProjectContext;
use crate::analyzer::fix;
use crate::analyzer::parser;

pub mod api;
pub mod cleanup;
pub mod control_flow;
pub mod helpers;
pub mod psr4;
pub mod sanity;
pub mod security;
pub mod strict_typing;

pub use api::{DeprecatedApiRule, InvalidThisRule};
pub use cleanup::{UnusedUseRule, UnusedVariableRule};
pub use control_flow::{
    DuplicateSwitchCaseRule, ImpossibleComparisonRule, RedundantConditionRule, UnreachableCodeRule,
};
pub use sanity::{ArrayKeyNotDefinedRule, DuplicateDeclarationRule, UndefinedVariableRule};
pub use security::{HardCodedCredentialsRule, IncludeUserInputRule, MutatingLiteralRule};
pub use strict_typing::{
    MissingArgumentRule, MissingReturnRule, StrictTypesRule, TypeMismatchRule,
};

pub trait DiagnosticRule {
    fn name(&self) -> &str;
    fn run(
        &self,
        parsed: &parser::ParsedSource,
        context: &ProjectContext,
    ) -> Vec<super::Diagnostic>;

    fn fix(&self, _parsed: &parser::ParsedSource, _context: &ProjectContext) -> Vec<fix::TextEdit> {
        Vec::new()
    }
}
