//! Entity parser
use crate::constants::entity::ENTITYNAME_SET;
use crate::parser::object;
use crate::parser::{MyExtra, NT, OSK};
use chumsky::prelude::*;

/// Entity parser
// PEG:
//    entity <- \NAME{} / \NAME &POST / \_SPACES
//    POST   <- EOL / (!alphabetic .)
pub(crate) fn entity_parser<'a, C: 'a>() -> impl Parser<'a, &'a str, NT, MyExtra<'a, C>> + Clone {
    // name := A string with a valid association in either org-entities or org-entities-user
    let name_parser = object::keyword_cs_parser(&ENTITYNAME_SET);
    let post_parser = any()
        .filter(|c: &char| !c.is_alphabetic())
        .or(just('\n'))
        .or(end().to('x'))      // why?
        ;

    just(r"\")
        .then(choice((
            name_parser
                .clone()
                .then(choice((
                    just("{}").to(true),            // \NAME {}
                    post_parser.rewind().to(false), // \NAME &POST
                )))
                .map(|(name, is_pattern2)| {
                    if is_pattern2 {
                        // (e.state() as &mut RollbackState<ParserState>).prev_char = Some('}');
                        vec![
                            crate::token!(OSK::EntityName, name),
                            crate::token!(OSK::LeftCurlyBracket, "{"),
                            crate::token!(OSK::RightCurlyBracket, "}"),
                        ]
                    } else {
                        // Pattern1: \NAME POST
                        // (e.state() as &mut RollbackState<ParserState>).prev_char =
                        //     name.chars().last();
                        vec![crate::token!(OSK::EntityName, name)]
                    }
                }),
            just("_")
                .then(just(" ").repeated().at_least(1).at_most(20).to_slice()) // \_SPACES
                .map(|(us, ws): (_, &str)| {
                    // (e.state() as &mut RollbackState<ParserState>).prev_char = ws.chars().last();
                    vec![
                        crate::token!(OSK::Underscore, us),
                        crate::token!(OSK::Spaces, ws),
                    ]
                }),
        )))
        .map(|(backslash, others)| {
            let mut children = Vec::with_capacity(1 + others.len());
            children.push(crate::token!(OSK::BackSlash, backslash));
            children.extend(others);

            crate::node!(OSK::Entity, children)
        })
        .boxed()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::common::{get_parser_output, get_parsers_output};
    use crate::parser::object;
    use pretty_assertions::assert_eq;

    #[test]
    fn test_01_name_post() {
        // \NAME POST(where POST=EOF)
        assert_eq!(
            get_parser_output(entity_parser::<()>(), r"\alpha"),
            r###"Entity@0..6
  BackSlash@0..1 "\\"
  EntityName@1..6 "alpha"
"###
        );

        assert_eq!(
            get_parsers_output(object::objects_parser::<()>(), r"\alpha foo \beta"),
            format!(
                r###"Root@0..16
  Entity@0..6
    BackSlash@0..1 "\\"
    EntityName@1..6 "alpha"
  Text@6..11 " foo "
  Entity@11..16
    BackSlash@11..12 "\\"
    EntityName@12..16 "beta"
"###
            )
        );

        assert_eq!(
            get_parsers_output(
                object::objects_parser::<()>(),
                r"\alpha
"
            ),
            format!(
                r###"Root@0..7
  Entity@0..6
    BackSlash@0..1 "\\"
    EntityName@1..6 "alpha"
  Text@6..7 "\n"
"###
            )
        );

        assert_eq!(
            get_parsers_output(object::objects_parser::<()>(), r"\alphafoo"),
            format!(
                r###"Root@0..9
  LatexFragment@0..9
    Text@0..9 "\\alphafoo"
"###
            )
        );
    }

    #[test]
    fn test_02_name_curly_bracket() {
        // \NAME{}
        assert_eq!(
            get_parser_output(entity_parser::<()>(), r"\beta{}"),
            r###"Entity@0..7
  BackSlash@0..1 "\\"
  EntityName@1..5 "beta"
  LeftCurlyBracket@5..6 "{"
  RightCurlyBracket@6..7 "}"
"###
        );

        assert_eq!(
            get_parsers_output(object::objects_parser::<()>(), r"\pi{}d"),
            r###"Root@0..6
  Entity@0..5
    BackSlash@0..1 "\\"
    EntityName@1..3 "pi"
    LeftCurlyBracket@3..4 "{"
    RightCurlyBracket@4..5 "}"
  Text@5..6 "d"
"###
        );

        assert_eq!(
            get_parsers_output(object::objects_parser::<()>(), r"\pid"),
            r###"Root@0..4
  LatexFragment@0..4
    Text@0..4 "\\pid"
"###
        );
    }

    #[test]
    fn test_03_spaces() {
        // \_SPACES
        assert_eq!(
            get_parser_output(entity_parser::<()>(), r"\_ "),
            r###"Entity@0..3
  BackSlash@0..1 "\\"
  Underscore@1..2 "_"
  Spaces@2..3 " "
"###
        );
        assert_eq!(
            get_parser_output(entity_parser::<()>(), r"\_          "),
            r###"Entity@0..12
  BackSlash@0..1 "\\"
  Underscore@1..2 "_"
  Spaces@2..12 "          "
"###
        );

        assert_eq!(
            get_parsers_output(object::objects_parser::<()>(), r"\_          \n"),
            r###"Root@0..14
  Entity@0..12
    BackSlash@0..1 "\\"
    Underscore@1..2 "_"
    Spaces@2..12 "          "
  LatexFragment@12..14
    Text@12..14 "\\n"
"###
        );
    }

    #[test]
    fn test_04_bad_entity() {
        assert_eq!(
            get_parsers_output(object::objects_parser::<()>(), r"\alphA \deltab "),
            r###"Root@0..15
  LatexFragment@0..6
    Text@0..6 "\\alphA"
  Text@6..7 " "
  LatexFragment@7..14
    Text@7..14 "\\deltab"
  Text@14..15 " "
"###
        );
    }

    #[test]
    fn test_05_entity() {
        // "\pi{}" should be parsed into "Entity(\pi{})", NOT "Entity(\pi) Text({})"
        assert_eq!(
            get_parser_output(entity_parser::<()>(), r"\pi{}"),
            r###"Entity@0..5
  BackSlash@0..1 "\\"
  EntityName@1..3 "pi"
  LeftCurlyBracket@3..4 "{"
  RightCurlyBracket@4..5 "}"
"###
        );
    }
}
