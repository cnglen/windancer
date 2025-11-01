//! Text markup parser, including bold, italic, underline, strikethrough, verbatim and code.
use crate::parser::ParserState;
use crate::parser::S2;
use crate::parser::syntax::OrgSyntaxKind;
use chumsky::inspector::RollbackState;
use chumsky::prelude::*;
use rowan::{GreenNode, GreenToken, NodeOrToken};
type NT = NodeOrToken<GreenNode, GreenToken>;
type OSK = OrgSyntaxKind;

/// text markup parser
pub(crate) fn text_markup_parser<'a>(
    object_parser: impl Parser<
        'a,
        &'a str,
        S2,
        extra::Full<Rich<'a, char>, RollbackState<ParserState>, ()>,
    > + Clone,
) -> impl Parser<'a, &'a str, S2, extra::Full<Rich<'a, char>, RollbackState<ParserState>, ()>> + Clone
{
    let post = one_of(" \t​-.,;:!?)}]\"'\\\r\n").or(end().to('x'));

    // a string may not begin or end with whitespace.
    let get_content = |marker: char| {
        none_of::<_, _, extra::Full<Rich<'_, char>, RollbackState<ParserState>, ()>>(" \t​")
            .then(any()
                  .and_is(just(marker).then(post).not().rewind())
                  .repeated()
                  .collect::<String>()
            )
            .try_map_with(|(first_char, remaining), e| {
                let pre_valid = e.state().prev_char.map_or(true, |c| {
                    matches!(
                        c,
                        ' '| '\t'| '​'|              // whitespace character
                        '-'| '('| '{'| '"'| '\''|
                        '\r'| '\n' // beginning of a line
                    )
                });

                let content = format!("{first_char}{remaining}");
                let content_end_valid = match content.chars().last() {
                    Some(c) if matches!(c, ' ' | '\t' | '​') => false,
                    _ => true
                };

                // println!("text-markup:content: pre_valid={pre_valid}, content_end_valid={content_end_valid}:\n  - content={content:?} Not valid if ends with whitesace\n  - PRE={:?} not valid if not in <whitespace and -({{ and others>", e.state().prev_char);

                match (pre_valid, content_end_valid) {
                    (true, true) => {Ok(())},

                    _ => {Err(Rich::custom(
                        e.span(),
                        format!("text-markup:content: pre_valid={pre_valid}, content_end_valid={content_end_valid}:\n  - content={content:?} ends with whitesace\n  - PRE={:?} not valid", e.state().prev_char),
                    ))}

                }
            })
            .to_slice()
    };

    let standard_objects_parser = object_parser
        .clone()
        .repeated()
        .at_least(1)
        .collect::<Vec<S2>>();

    let bold = just("*")
        .then(standard_objects_parser.clone().nested_in(get_content('*')))
        .then(just("*"))
        .then_ignore(post.rewind())
        .try_map_with(|((start_marker, content), end_marker), e| {
            // pre valid should NOT be deteced here, state.prev_char is update by standard object parser
            // let old_state = e.state().prev_char;

            e.state().prev_char = end_marker.chars().last();
            // println!("bold: {:?}{:?}{:?}, set prev_char {:?} -> {:?}", start_marker, content, end_marker, old_state, e.state().prev_char);

            let mut children = vec![];
            children.push(NT::Token(GreenToken::new(
                OSK::Asterisk.into(),
                start_marker,
            )));
            for node in content {
                match node {
                    S2::Single(e) => {
                        children.push(e);
                    }
                    S2::Double(e1, e2) => {
                        children.push(e1);
                        children.push(e2);
                    }
                    _ => {}
                }
            }
            children.push(NT::Token(GreenToken::new(OSK::Asterisk.into(), end_marker)));

            Ok(S2::Single(NT::Node(GreenNode::new(
                OSK::Bold.into(),
                children,
            ))))
        });

    let italic = just("/")
        .then(standard_objects_parser.clone().nested_in(get_content('/'))) // 这里objects_parser可能会执行plain_text_parser, 会更新prev_char!!(不应更新)
        .then(just("/"))
        .then_ignore(post.rewind())
        .try_map_with(|((start_marker, content), end_marker), e| {
            // pre valid should NOT be deteced here, state.prev_char is update by standard object parser
            e.state().prev_char = end_marker.chars().last();

            let mut children = vec![];
            children.push(NT::Token(GreenToken::new(OSK::Slash.into(), start_marker)));
            for node in content {
                match node {
                    S2::Single(e) => {
                        children.push(e);
                    }
                    S2::Double(e1, e2) => {
                        children.push(e1);
                        children.push(e2);
                    }
                    _ => {}
                }
            }
            children.push(NT::Token(GreenToken::new(OSK::Slash.into(), end_marker)));

            Ok(S2::Single(NT::Node(GreenNode::new(
                OSK::Italic.into(),
                children,
            ))))
        });

    let underline = just("_")
        .then(standard_objects_parser.clone().nested_in(get_content('_')))
        .then(just("_"))
        .then_ignore(post.rewind())
        .try_map_with(|((start_marker, content), end_marker), e| {
            // pre valid should NOT be deteced here, state.prev_char is update by standard object parser
            e.state().prev_char = end_marker.chars().last();

            let mut children = vec![];
            children.push(NT::Token(GreenToken::new(
                OSK::Underscore.into(),
                start_marker,
            )));
            for node in content {
                match node {
                    S2::Single(e) => {
                        children.push(e);
                    }
                    S2::Double(e1, e2) => {
                        children.push(e1);
                        children.push(e2);
                    }
                    _ => {}
                }
            }
            children.push(NT::Token(GreenToken::new(
                OSK::Underscore.into(),
                end_marker,
            )));

            Ok(S2::Single(NT::Node(GreenNode::new(
                OSK::Underline.into(),
                children,
            ))))
        });

    let strikethrough = just("+")
        .then(standard_objects_parser.clone().nested_in(get_content('+')))
        .then(just("+"))
        .then_ignore(post.rewind())
        .try_map_with(|((start_marker, content), end_marker), e| {
            // pre valid should NOT be deteced here, state.prev_char is update by standard object parser
            e.state().prev_char = end_marker.chars().last();

            let mut children = vec![];
            children.push(NT::Token(GreenToken::new(OSK::Plus.into(), start_marker)));
            for node in content {
                match node {
                    S2::Single(e) => {
                        children.push(e);
                    }
                    S2::Double(e1, e2) => {
                        children.push(e1);
                        children.push(e2);
                    }
                    _ => {}
                }
            }
            children.push(NT::Token(GreenToken::new(OSK::Plus.into(), end_marker)));

            Ok(S2::Single(NT::Node(GreenNode::new(
                OSK::Strikethrough.into(),
                children,
            ))))
        });

    let code = just::<_, _, extra::Full<Rich<'_, char>, RollbackState<ParserState>, ()>>("~")
        .then(get_content('~'))
        .then(just("~"))
        .then_ignore(post.rewind())
        .try_map_with(|((start_marker, content), end_marker), e| {
            e.state().prev_char = end_marker.chars().last();

            let mut children = vec![];
            children.push(NT::Token(GreenToken::new(OSK::Tilde.into(), start_marker)));
            children.push(NT::Token(GreenToken::new(OSK::Text.into(), content)));
            children.push(NT::Token(GreenToken::new(OSK::Tilde.into(), end_marker)));

            Ok(S2::Single(NT::Node(GreenNode::new(
                OSK::Code.into(),
                children,
            ))))
        });

    let verbatim = just::<_, _, extra::Full<Rich<'_, char>, RollbackState<ParserState>, ()>>("=")
        .then(get_content('='))
        .then(just("="))
        .then_ignore(post.rewind())
        .try_map_with(|((start_marker, content), end_marker), e| {
            e.state().prev_char = end_marker.chars().last();

            let mut children = vec![];
            children.push(NT::Token(GreenToken::new(OSK::Equals.into(), start_marker)));
            children.push(NT::Token(GreenToken::new(OSK::Text.into(), content)));
            children.push(NT::Token(GreenToken::new(OSK::Equals.into(), end_marker)));

            Ok(S2::Single(NT::Node(GreenNode::new(
                OSK::Verbatim.into(),
                children,
            ))))
        });

    bold.or(italic)
        .or(underline)
        .or(strikethrough)
        .or(verbatim)
        .or(code)
}

#[cfg(test)]
mod tests {
    use crate::parser::common::get_parsers_output;
    use crate::parser::object;
    use pretty_assertions::assert_eq; // 该包仅能用于测试

    #[test]
    fn test_markup_01_basic_en() {
        assert_eq!(
            get_parsers_output(
                object::objects_parser(),
                "a *bold*, a /italic/, a _underline_, a +strikethrough+, a ~code~, and a =verbatim= text"
            ),
            r##"Root@0..87
  Text@0..2 "a "
  Bold@2..8
    Asterisk@2..3 "*"
    Text@3..7 "bold"
    Asterisk@7..8 "*"
  Text@8..12 ", a "
  Italic@12..20
    Slash@12..13 "/"
    Text@13..19 "italic"
    Slash@19..20 "/"
  Text@20..24 ", a "
  Underline@24..35
    Underscore@24..25 "_"
    Text@25..34 "underline"
    Underscore@34..35 "_"
  Text@35..39 ", a "
  Strikethrough@39..54
    Plus@39..40 "+"
    Text@40..53 "strikethrough"
    Plus@53..54 "+"
  Text@54..58 ", a "
  Code@58..64
    Tilde@58..59 "~"
    Text@59..63 "code"
    Tilde@63..64 "~"
  Text@64..72 ", and a "
  Verbatim@72..82
    Equals@72..73 "="
    Text@73..81 "verbatim"
    Equals@81..82 "="
  Text@82..87 " text"
"##
        );
    }

    #[test]
    fn test_markup_02_basic_cn() {
        assert_eq!(
            get_parsers_output(
                object::objects_parser(),
                "一个​*粗体*​、​/斜体/​、​_下划线_​、​+横划线+​、​~编程~​和​=字面=​文本"
            ),
            r##"Root@0..117
  Text@0..9 "一个\u{200b}"
  Bold@9..17
    Asterisk@9..10 "*"
    Text@10..16 "粗体"
    Asterisk@16..17 "*"
  Text@17..26 "\u{200b}、\u{200b}"
  Italic@26..34
    Slash@26..27 "/"
    Text@27..33 "斜体"
    Slash@33..34 "/"
  Text@34..43 "\u{200b}、\u{200b}"
  Underline@43..54
    Underscore@43..44 "_"
    Text@44..53 "下划线"
    Underscore@53..54 "_"
  Text@54..63 "\u{200b}、\u{200b}"
  Strikethrough@63..74
    Plus@63..64 "+"
    Text@64..73 "横划线"
    Plus@73..74 "+"
  Text@74..83 "\u{200b}、\u{200b}"
  Code@83..91
    Tilde@83..84 "~"
    Text@84..90 "编程"
    Tilde@90..91 "~"
  Text@91..100 "\u{200b}和\u{200b}"
  Verbatim@100..108
    Equals@100..101 "="
    Text@101..107 "字面"
    Equals@107..108 "="
  Text@108..117 "\u{200b}文本"
"##
        );
    }

    #[test]
    fn test_markup_03_basic_negative() {
        assert_eq!(
            get_parsers_output(object::objects_parser(), "Not*bold* bad PRE"),
            r##"Root@0..17
  Text@0..17 "Not*bold* bad PRE"
"##
        );

        assert_eq!(
            get_parsers_output(object::objects_parser(), "Not *bold*( bad POST"),
            r##"Root@0..20
  Text@0..20 "Not *bold*( bad POST"
"##
        );

        assert_eq!(
            get_parsers_output(object::objects_parser(), "Not * bold* *bold * bad content"),
            r##"Root@0..31
  Text@0..31 "Not * bold* *bold * b ..."
"##
        );
    }

    #[test]
    fn test_markup_04_nested() {
        assert_eq!(
            get_parsers_output(object::objects_parser(), "*/_+=all=+_/*"),
            r##"Root@0..13
  Bold@0..13
    Asterisk@0..1 "*"
    Italic@1..12
      Slash@1..2 "/"
      Underline@2..11
        Underscore@2..3 "_"
        Strikethrough@3..10
          Plus@3..4 "+"
          Verbatim@4..9
            Equals@4..5 "="
            Text@5..8 "all"
            Equals@8..9 "="
          Plus@9..10 "+"
        Underscore@10..11 "_"
      Slash@11..12 "/"
    Asterisk@12..13 "*"
"##
        );

        assert_eq!(
            get_parsers_output(object::objects_parser(), r"//ab//"),
            r##"Root@0..6
  Italic@0..6
    Slash@0..1 "/"
    Italic@1..5
      Slash@1..2 "/"
      Text@2..4 "ab"
      Slash@4..5 "/"
    Slash@5..6 "/"
"##
        );
    }

    #[test]
    fn test_markup_05_nested() {
        assert_eq!(
            get_parsers_output(object::objects_parser(), "~=*_/inner-most/_*=~"),
            r##"Root@0..20
  Code@0..20
    Tilde@0..1 "~"
    Text@1..19 "=*_/inner-most/_*="
    Tilde@19..20 "~"
"##
        );
    }

    #[test]
    fn test_markup_06_nested() {
        assert_eq!(
            get_parsers_output(
                object::objects_parser(),
                r##"a */bold-italic/* text
a *bold NOT/italic/* text
*/This text is bold and italic, _and this part is also underlined_./*
a */bold-italic/ *bold*
"##
            ),
            r##"Root@0..143
  Text@0..2 "a "
  Bold@2..17
    Asterisk@2..3 "*"
    Italic@3..16
      Slash@3..4 "/"
      Text@4..15 "bold-italic"
      Slash@15..16 "/"
    Asterisk@16..17 "*"
  Text@17..25 " text\na "
  Bold@25..43
    Asterisk@25..26 "*"
    Text@26..42 "bold NOT/italic/"
    Asterisk@42..43 "*"
  Text@43..49 " text\n"
  Bold@49..118
    Asterisk@49..50 "*"
    Italic@50..117
      Slash@50..51 "/"
      Text@51..81 "This text is bold and ..."
      Underline@81..115
        Underscore@81..82 "_"
        Text@82..114 "and this part is also ..."
        Underscore@114..115 "_"
      Text@115..116 "."
      Slash@116..117 "/"
    Asterisk@117..118 "*"
  Text@118..121 "\na "
  Bold@121..142
    Asterisk@121..122 "*"
    Italic@122..135
      Slash@122..123 "/"
      Text@123..134 "bold-italic"
      Slash@134..135 "/"
    Text@135..141 " *bold"
    Asterisk@141..142 "*"
  Text@142..143 "\n"
"##
        );
    }

    #[test]
    fn test_markup_07_nested() {
        assert_eq!(
            get_parsers_output(
                object::objects_parser(),
                r##"*/_bold-italic-underline_/*

*_/bold-underline-italic/_*

_/*underline-italic-bold*/_

_*/underline-bold-italic/*_

/_*italic-underline-bold*_/

/*_italic-bold-underline_*/

+/*_strikethrough-italic-bold-underline_*/+

+/_*strikethrough-italic-underline-bold*_/+

+/_*strikethrough-italic_*/+

*_~inner-most~_*

*_~=inner-most-include-equal=~_*

*_=~inner-most-include-tilde~=_*    

~=*_/inner-most-include-equal-star-underscore-slash/_*=~
"##
            ),
            r##"Root@0..441
  Bold@0..27
    Asterisk@0..1 "*"
    Italic@1..26
      Slash@1..2 "/"
      Underline@2..25
        Underscore@2..3 "_"
        Text@3..24 "bold-italic-underline"
        Underscore@24..25 "_"
      Slash@25..26 "/"
    Asterisk@26..27 "*"
  Text@27..29 "\n\n"
  Bold@29..56
    Asterisk@29..30 "*"
    Underline@30..55
      Underscore@30..31 "_"
      Italic@31..54
        Slash@31..32 "/"
        Text@32..53 "bold-underline-italic"
        Slash@53..54 "/"
      Underscore@54..55 "_"
    Asterisk@55..56 "*"
  Text@56..58 "\n\n"
  Underline@58..85
    Underscore@58..59 "_"
    Italic@59..84
      Slash@59..60 "/"
      Bold@60..83
        Asterisk@60..61 "*"
        Text@61..82 "underline-italic-bold"
        Asterisk@82..83 "*"
      Slash@83..84 "/"
    Underscore@84..85 "_"
  Text@85..87 "\n\n"
  Underline@87..114
    Underscore@87..88 "_"
    Bold@88..113
      Asterisk@88..89 "*"
      Italic@89..112
        Slash@89..90 "/"
        Text@90..111 "underline-bold-italic"
        Slash@111..112 "/"
      Asterisk@112..113 "*"
    Underscore@113..114 "_"
  Text@114..116 "\n\n"
  Italic@116..143
    Slash@116..117 "/"
    Underline@117..142
      Underscore@117..118 "_"
      Bold@118..141
        Asterisk@118..119 "*"
        Text@119..140 "italic-underline-bold"
        Asterisk@140..141 "*"
      Underscore@141..142 "_"
    Slash@142..143 "/"
  Text@143..145 "\n\n"
  Italic@145..172
    Slash@145..146 "/"
    Bold@146..171
      Asterisk@146..147 "*"
      Underline@147..170
        Underscore@147..148 "_"
        Text@148..169 "italic-bold-underline"
        Underscore@169..170 "_"
      Asterisk@170..171 "*"
    Slash@171..172 "/"
  Text@172..174 "\n\n"
  Strikethrough@174..217
    Plus@174..175 "+"
    Italic@175..216
      Slash@175..176 "/"
      Bold@176..215
        Asterisk@176..177 "*"
        Underline@177..214
          Underscore@177..178 "_"
          Text@178..213 "strikethrough-italic- ..."
          Underscore@213..214 "_"
        Asterisk@214..215 "*"
      Slash@215..216 "/"
    Plus@216..217 "+"
  Text@217..219 "\n\n"
  Strikethrough@219..262
    Plus@219..220 "+"
    Italic@220..261
      Slash@220..221 "/"
      Underline@221..260
        Underscore@221..222 "_"
        Bold@222..259
          Asterisk@222..223 "*"
          Text@223..258 "strikethrough-italic- ..."
          Asterisk@258..259 "*"
        Underscore@259..260 "_"
      Slash@260..261 "/"
    Plus@261..262 "+"
  Text@262..264 "\n\n"
  Strikethrough@264..292
    Plus@264..265 "+"
    Italic@265..291
      Slash@265..266 "/"
      Subscript@266..268
        Caret@266..267 "_"
        Text@267..268 "*"
      Text@268..288 "strikethrough-italic"
      Subscript@288..290
        Caret@288..289 "_"
        Text@289..290 "*"
      Slash@290..291 "/"
    Plus@291..292 "+"
  Text@292..294 "\n\n"
  Bold@294..310
    Asterisk@294..295 "*"
    Underline@295..309
      Underscore@295..296 "_"
      Code@296..308
        Tilde@296..297 "~"
        Text@297..307 "inner-most"
        Tilde@307..308 "~"
      Underscore@308..309 "_"
    Asterisk@309..310 "*"
  Text@310..312 "\n\n"
  Bold@312..344
    Asterisk@312..313 "*"
    Underline@313..343
      Underscore@313..314 "_"
      Code@314..342
        Tilde@314..315 "~"
        Text@315..341 "=inner-most-include-e ..."
        Tilde@341..342 "~"
      Underscore@342..343 "_"
    Asterisk@343..344 "*"
  Text@344..346 "\n\n"
  Bold@346..378
    Asterisk@346..347 "*"
    Underline@347..377
      Underscore@347..348 "_"
      Verbatim@348..376
        Equals@348..349 "="
        Text@349..375 "~inner-most-include-t ..."
        Equals@375..376 "="
      Underscore@376..377 "_"
    Asterisk@377..378 "*"
  Text@378..384 "    \n\n"
  Code@384..440
    Tilde@384..385 "~"
    Text@385..439 "=*_/inner-most-includ ..."
    Tilde@439..440 "~"
  Text@440..441 "\n"
"##
        )
    }

    #[test]
    fn test_markup_08_nested() {
        assert_eq!(
            get_parsers_output(object::objects_parser(), r##" */abc/ "##),
            r##"Root@0..8
  Text@0..8 " */abc/ "
"##
        );

        assert_eq!(
            get_parsers_output(object::objects_parser(), r##" */abc/ _adf_"##),
            r##"Root@0..13
  Text@0..8 " */abc/ "
  Underline@8..13
    Underscore@8..9 "_"
    Text@9..12 "adf"
    Underscore@12..13 "_"
"##
        );

        assert_eq!(
            get_parsers_output(object::objects_parser(), r##" */_abc/* bar_"##),
            r##"Root@0..14
  Text@0..1 " "
  Bold@1..9
    Asterisk@1..2 "*"
    Italic@2..8
      Slash@2..3 "/"
      Text@3..7 "_abc"
      Slash@7..8 "/"
    Asterisk@8..9 "*"
  Text@9..14 " bar_"
"##
        );

        assert_eq!(
            get_parsers_output(object::objects_parser(), r##" /*+/"##),
            r##"Root@0..5
  Text@0..1 " "
  Italic@1..5
    Slash@1..2 "/"
    Text@2..4 "*+"
    Slash@4..5 "/"
"##
        );

        assert_eq!(
            get_parsers_output(object::objects_parser(), r##" ** a **"##),
            r##"Root@0..8
  Text@0..1 " "
  Bold@1..8
    Asterisk@1..2 "*"
    Text@2..7 "* a *"
    Asterisk@7..8 "*"
"##
        );

        assert_eq!(
            get_parsers_output(object::objects_parser(), r##" **a bold** : 2b"##),
            r##"Root@0..16
  Text@0..1 " "
  Bold@1..11
    Asterisk@1..2 "*"
    Bold@2..10
      Asterisk@2..3 "*"
      Text@3..9 "a bold"
      Asterisk@9..10 "*"
    Asterisk@10..11 "*"
  Text@11..16 " : 2b"
"##
        );

        assert_eq!(
            get_parsers_output(object::objects_parser(), r##" ***a bold** : 2b"##),
            r##"Root@0..17
  Text@0..1 " "
  Bold@1..12
    Asterisk@1..2 "*"
    Bold@2..11
      Asterisk@2..3 "*"
      Text@3..10 "*a bold"
      Asterisk@10..11 "*"
    Asterisk@11..12 "*"
  Text@12..17 " : 2b"
"##
        );

        assert_eq!(
            get_parsers_output(object::objects_parser(), r##" ***a bold*** : 3b"##),
            r##"Root@0..18
  Text@0..1 " "
  Bold@1..13
    Asterisk@1..2 "*"
    Bold@2..12
      Asterisk@2..3 "*"
      Bold@3..11
        Asterisk@3..4 "*"
        Text@4..10 "a bold"
        Asterisk@10..11 "*"
      Asterisk@11..12 "*"
    Asterisk@12..13 "*"
  Text@13..18 " : 3b"
"##
        );
    }

    #[test]
    fn test_markup_09_code() {
        assert_eq!(
            get_parsers_output(object::objects_parser(), "~code~"),
            r##"Root@0..6
  Code@0..6
    Tilde@0..1 "~"
    Text@1..5 "code"
    Tilde@5..6 "~"
"##
        );

        assert_eq!(
            get_parsers_output(object::objects_parser(), "~code ~end~"),
            r##"Root@0..11
  Code@0..11
    Tilde@0..1 "~"
    Text@1..10 "code ~end"
    Tilde@10..11 "~"
"##
        );

        assert_eq!(
            get_parsers_output(object::objects_parser(), "~code end~ other~"),
            r##"Root@0..17
  Code@0..10
    Tilde@0..1 "~"
    Text@1..9 "code end"
    Tilde@9..10 "~"
  Text@10..17 " other~"
"##
        );

        assert_eq!(
            get_parsers_output(object::objects_parser(), "~~code end~ other~"),
            r##"Root@0..18
  Code@0..11
    Tilde@0..1 "~"
    Text@1..10 "~code end"
    Tilde@10..11 "~"
  Text@11..18 " other~"
"##
        );
    }

    #[test]
    fn test_markup_10_bad_nested() {
        assert_eq!(
            get_parsers_output(
                object::objects_parser(),
                "_underline_ */_underline_ italic/"
            ),
            r##"Root@0..33
  Underline@0..11
    Underscore@0..1 "_"
    Text@1..10 "underline"
    Underscore@10..11 "_"
  Text@11..14 " */"
  Subscript@14..24
    Caret@14..15 "_"
    Text@15..24 "underline"
  Text@24..33 "_ italic/"
"##
        );
    }

    #[test]
    fn test_markup_11_object() {
        assert_eq!(
            get_parsers_output(
                object::objects_parser(),
                "a *[[https://www.foo.org][foo]]* link"
            ),
            r##"Root@0..37
  Text@0..2 "a "
  Bold@2..32
    Asterisk@2..3 "*"
    Link@3..31
      LeftSquareBracket@3..4 "["
      LinkPath@4..25
        LeftSquareBracket@4..5 "["
        Text@5..24 "https://www.foo.org"
        RightSquareBracket@24..25 "]"
      LinkDescription@25..30
        LeftSquareBracket@25..26 "["
        Text@26..29 "foo"
        RightSquareBracket@29..30 "]"
      RightSquareBracket@30..31 "]"
    Asterisk@31..32 "*"
  Text@32..37 " link"
"##
        );
    }
}
