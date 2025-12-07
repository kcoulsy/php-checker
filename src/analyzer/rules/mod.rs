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
#[cfg(test)]
pub mod test_utils;

pub use api::{DeprecatedApiRule, InvalidThisRule};
pub use cleanup::{UnusedUseRule, UnusedVariableRule};
pub use control_flow::{
    DuplicateSwitchCaseRule, FallthroughRule, ImpossibleComparisonRule, RedundantConditionRule,
    UnreachableCodeRule, UnreachableStatementRule,
};
pub use sanity::{ArrayKeyNotDefinedRule, DuplicateDeclarationRule, UndefinedVariableRule};
pub use security::{
    HardCodedCredentialsRule, HardCodedKeysRule, IncludeUserInputRule, MutatingLiteralRule,
    WeakHashingRule,
};
pub use strict_typing::{
    ConsistentReturnRule, ForceReturnTypeRule, MissingArgumentRule, MissingReturnRule,
    PhpDocParamCheckRule, PhpDocReturnCheckRule, PhpDocReturnValueCheckRule, PhpDocVarCheckRule,
    StrictTypesRule, TypeMismatchRule,
};

pub trait DiagnosticRule: Send + Sync {
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
