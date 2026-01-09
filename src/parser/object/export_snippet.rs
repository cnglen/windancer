//! export snippet
use crate::parser::{MyExtra, NT, OSK};
use chumsky::prelude::*;

// PEG: export_snippet <- "@@" BACKEND ":" VALUE? "@@"
pub(crate) fn export_snippet_parser<'a, C: 'a>()
-> impl Parser<'a, &'a str, NT, MyExtra<'a, C>> + Clone {
    let backend = any()
        .filter(|c: &char| c.is_alphanumeric() || matches!(c, '-'))
        .repeated()
        .at_least(1)
        .to_slice();

    let value = any()
        .and_is(just("@@").not())
        .repeated()
        .at_least(1)
        .to_slice();

    group((just("@@"), backend, just(":"), value.or_not(), just("@@"))).map(
        |(begin_at2, backend, colon, maybe_value, end_at2)| {
            let mut children = Vec::with_capacity(5);
            children.push(crate::token!(OSK::At2, begin_at2));
            children.push(crate::token!(OSK::ExportSnippetBackend, backend));
            children.push(crate::token!(OSK::Colon, colon));
            if let Some(value) = maybe_value {
                children.push(crate::token!(OSK::ExportSnippetValue, value));
            }
            children.push(crate::token!(OSK::At2, end_at2));

            crate::node!(OSK::ExportSnippet, children)
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::common::get_parser_output;
    use pretty_assertions::assert_eq;

    #[test]
    fn test_export_snippet_01() {
        let input = r##"@@html:<b>@@"##;
        let expected_output = r##"ExportSnippet@0..12
  At2@0..2 "@@"
  ExportSnippetBackend@2..6 "html"
  Colon@6..7 ":"
  ExportSnippetValue@7..10 "<b>"
  At2@10..12 "@@"
"##;
        assert_eq!(
            get_parser_output(export_snippet_parser::<()>(), input),
            expected_output,
        );
    }

    #[test]
    fn test_export_snippet_02() {
        let input = r##"@@beamer:some code@@"##;
        let expected_output = r##"ExportSnippet@0..20
  At2@0..2 "@@"
  ExportSnippetBackend@2..8 "beamer"
  Colon@8..9 ":"
  ExportSnippetValue@9..18 "some code"
  At2@18..20 "@@"
"##;
        assert_eq!(
            get_parser_output(export_snippet_parser::<()>(), input),
            expected_output,
        );
    }
}
