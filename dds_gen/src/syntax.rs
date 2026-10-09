//! Optional string-only syntax API for source adapters.
//!
//! This fork API parses the complete input without preprocessing, file access,
//! code generation or DDS entities. It performs syntax parsing only; callers
//! own semantic validation and resource limits. Grammar rules may change when
//! the pinned fork revision changes.

use crate::parser::IdlParser;
pub use crate::parser::Rule;
use pest::Parser;

/// One syntax node borrowing its source text and byte span.
pub type Node<'a> = pest::iterators::Pair<'a, Rule>;
/// Syntax nodes borrowing their source text and byte spans.
pub type Nodes<'a> = pest::iterators::Pairs<'a, Rule>;

/// Parses a complete IDL specification without invoking the preprocessor or generator.
pub fn parse(source: &str) -> Result<Nodes<'_>, pest::error::Error<Rule>> {
    IdlParser::parse(Rule::specification, source)
}
