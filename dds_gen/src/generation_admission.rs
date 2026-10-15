//! Syntax-only placements are not generated API or Member ID implementations.
use crate::parser::Rule;
use pest::iterators::Pair;

pub(crate) fn validate(specification: Pair<'_, Rule>) -> Result<(), String> {
    for node in specification.into_inner().flatten() {
        if matches!(node.as_rule(), Rule::member | Rule::element_spec) {
            if let Some(definition) = node.clone().into_inner().find(|n| {
                matches!(
                    n.as_rule(),
                    Rule::struct_def | Rule::union_def | Rule::enum_dcl
                )
            }) {
                let (line, column) = definition.as_span().start_pos().line_col();
                return Err(format!(
                    "Aggregate-local definition on {:?} is available for syntax inspection but not code generation at {line}:{column}",
                    node.as_rule()
                ));
            }
        }
        if !matches!(
            node.as_rule(),
            Rule::module_dcl | Rule::element_spec | Rule::switch_type_spec
        ) {
            continue;
        }
        if let Some(annotation) = node
            .clone()
            .into_inner()
            .find(|n| n.as_rule() == Rule::annotation_appl)
        {
            let (line, column) = annotation.as_span().start_pos().line_col();
            return Err(format!(
                "Annotation on {:?} is available for syntax inspection but not code generation at {line}:{column}",
                node.as_rule()
            ));
        }
    }
    Ok(())
}
