pub mod extractor;
pub mod parser;
pub mod types;

pub use extractor::{extract_phpdoc_for_node, find_preceding_comment};
pub use parser::{PhpDocComment, PhpDocParser};
pub use types::{MethodTag, ParamTag, PropertyTag, ReturnTag, ThrowsTag, TypeExpression, VarTag};
