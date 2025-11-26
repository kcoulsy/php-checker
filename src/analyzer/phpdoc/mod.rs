pub mod extractor;
pub mod parser;
pub mod types;

pub use extractor::{extract_phpdoc_for_node, find_preceding_comment};
pub use parser::{PhpDocParser, PhpDocComment};
pub use types::{TypeExpression, ParamTag, ReturnTag, VarTag, ThrowsTag};
