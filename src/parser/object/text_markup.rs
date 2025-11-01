//! Text markup parser, including bold, italic, underline, strikethrough, verbatim and code.
use crate::parser::ParserState;
use crate::parser::S2;
use crate::parser::syntax::OrgSyntaxKind;
use chumsky::input::InputRef;
use chumsky::inspector::RollbackState;
use chumsky::prelude::*;
use phf::{phf_map, phf_set};
use rowan::{GreenNode, GreenToken, NodeOrToken};
use std::collections::HashMap;
use std::ops::Range;
type NT = NodeOrToken<GreenNode, GreenToken>;
type OSK = OrgSyntaxKind;

// PRE: Either a whitespace character, -, (, {, ', ", or the beginning of a line.
// FIXME: begin of file(BOF) is NOT included
static PRE_SET_WITHOUT_BOF: phf::Set<char> = phf_set! {
    ' ', '\t', '​',              // whitespace character
    '-', '(', '{', '"', '\'',
    '\r', '\n'                  // beginning of a line
};

// Either a whitespace character, -, ., ,, ;, :, !, ?, ', ), }, [, ", \ (backslash), or the end of a line.
static POST_SET: phf::Set<char> = phf_set! {
    ' ', '\t', '​',              // whitespace character
    '-', '.', ',', ';', ':', '!', '?', ')', '}', ']', '"', '\'', '\\',
    '\r', '\n'                  // end of a line
};

static MARKER_SET: phf::Set<char> = phf_set! {'*', '/', '_', '+', '~', '='};
static MARKER_SET_HIGH_PRIORITY: phf::Set<char> = phf_set! {'~', '='};
static WHITESPACE_SET: phf::Set<char> = phf_set! {' ', '\t', '​'};
static END_OF_LINE_SET: phf::Set<char> = phf_set! {'\r', '\n'};

/// char:=marker_char, usize:=index of marker char
struct MarkerStack {
    data: Vec<(char, usize)>,
}

impl MarkerStack {
    fn last(&self) -> Option<&(char, usize)> {
        self.data.last()
    }

    fn push(&mut self, x: (char, usize)) {
        self.data.push(x);
    }

    fn pop(&mut self) -> Option<(char, usize)> {
        self.data.pop()
    }

    fn len(&self) -> usize {
        self.data.len()
    }

    fn history_state_without_last(&self) -> Vec<char> {
        match self.data.len() > 0 {
            true => self.data[0..self.data.len() - 1]
                .iter()
                .map(|&(state, _)| state)
                .collect(),

            false => vec![],
        }
    }

    fn contains(&self, x: &(char, usize)) -> bool {
        self.data.contains(x)
    }
}
impl Default for MarkerStack {
    fn default() -> Self {
        Self { data: vec![] }
    }
}

/// The middle result of preprodessor
#[derive(Debug)]
struct Result {
    data: Vec<(usize, String)>,
    i2rep: HashMap<usize, String>,
}

impl Default for Result {
    fn default() -> Self {
        Self {
            data: vec![],
            i2rep: HashMap::new(),
        }
    }
}

impl Result {
    fn len(&self) -> usize {
        self.data.len()
    }

    fn last(&self) -> Option<&(usize, String)> {
        self.data.last()
    }

    fn pop(&mut self) -> Option<(usize, String)> {
        let ans = self.data.pop();
        if let Some((i, _)) = ans {
            self.i2rep.remove(&i);
        }
        ans
    }

    fn push(&mut self, i: usize, rep: String) {
        self.data.push((i, rep.clone()));
        self.i2rep.insert(i, rep);
    }

    fn get(&self, i: &usize) -> Option<&String> {
        self.i2rep.get(i)
    }

    fn remove(&mut self, i: &usize) -> Option<String> {
        self.i2rep.remove(i)
    }
    fn contains(&self, i: &usize) -> bool {
        self.i2rep.contains_key(&i)
    }
}

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
        .map(|s|{println!("s0={s:?}"); s})
        .then(standard_objects_parser.clone().nested_in(get_content('*')))
        .then(just("*"))
        .then_ignore(post.rewind())
        .try_map_with(|((start_marker, content), end_marker), e| {
            // pre valid should NOT be deteced here, state.prev_char is update by standard object parser
            let old_state = e.state().prev_char;

            e.state().prev_char = end_marker.chars().last();
            println!("bold: {:?}{:?}{:?}, set prev_char {:?} -> {:?}", start_marker, content, end_marker, old_state, e.state().prev_char);
                
            let mut children = vec![];
            children.push(NT::Token(GreenToken::new(OSK::Asterisk.into(), start_marker)));
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
            children.push(NT::Token(GreenToken::new(OSK::Underscore.into(), start_marker)));
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
            children.push(NT::Token(GreenToken::new(OSK::Underscore.into(), end_marker)));

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
    
    bold
        .or(italic)
        .or(underline)
        .or(strikethrough)
        .or(verbatim)
        .or(code)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::common::{get_parser_output, get_parsers_output};
    use crate::parser::object;
    use pretty_assertions::assert_eq; // 该包仅能用于测试

    #[test]
    fn test_01_nested_markup() {
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
    }

    #[test]
    fn test_02_nested_markup() {
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
    fn test_markup_basic() {
        assert_eq!(get_parsers_output(object::objects_parser(), "a *bold*, a /italic/, a _underline_, a +strikethrough+, a ~code~, and a =verbatim= text"), r##"Root@0..87
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
"##);
    }

        
    #[test]
    fn test_code() {
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
    fn test_bold() {
        assert_eq!(
            get_parsers_output(object::objects_parser(), "*bold*"),
            r##"Root@0..6
  Bold@0..6
    Asterisk@0..1 "*"
    Text@1..5 "bold"
    Asterisk@5..6 "*"
"##
        );
    }

    #[test]
    fn test_02a_nested_markup() {

        //         assert_eq!(
        //             get_parsers_output(object::objects_parser(), "*=inner-most=*"),
        //             r##"Root@0..14
        //   Bold@0..14
        //     Tilde@0..1 "*"
        //     Text@1..19 "=*_/inner-most/_*="
        //     Tilde@13..14 "*"
        // "##
        //         );
    }

    #[test]
    fn test_03_bad_nested_markup() {
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
}
