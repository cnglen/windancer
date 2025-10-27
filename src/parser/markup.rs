//! Text markup parser, including bold, italic, underline, strikethrough, verbatim and code.
use crate::parser::ParserState;
use crate::parser::S2;
use crate::parser::object::text_parser;
use crate::parser::syntax::OrgSyntaxKind;
use chumsky::input::InputRef;
use chumsky::inspector::SimpleState;
use chumsky::prelude::*;
use phf::{phf_map, phf_set};
use rowan::{GreenNode, GreenToken, NodeOrToken};
use std::collections::{HashMap, HashSet};
use std::ops::Range;

static NORMAL_PRE: phf::Set<char> = phf_set! {' ', '\t', '(', '{', '"', '\'', '​'};
static NORMAL_POST: phf::Set<char> = phf_set! {' ', '\t', '.', ',', ';', ':', '!', '?', ')', '}', ']', '"', '\'', '\\', '\r', '\n', '​'};

static MARKER_SET: phf::Set<char> = phf_set! {'*', '/', '_', '+', '~', '='};
static MARKER_SET_HIGH_PRIORITY: phf::Set<char> = phf_set! {'~', '='};
static WHITESPACE_SET: phf::Set<char> = phf_set! {' ', '\t', '​'};
static END_OF_LINE_SET: phf::Set<char> = phf_set! {'\r', '\n'};

// FIXME: how to avoid conflict?
static CHAR2TYPE_BEGIN: phf::Map<char, &'static str> = phf_map! {
    '*' => "<bold_314159265358979323846264338327950288419716939937510>",
    '/' => "<italic_314159265358979323846264338327950288419716939937510>",
    '_' => "<underline_314159265358979323846264338327950288419716939937510>",
    '+' => "<strikethrough_314159265358979323846264338327950288419716939937510>",
    '=' => "<verbatim_314159265358979323846264338327950288419716939937510>",
    '~' => "<code_314159265358979323846264338327950288419716939937510>"
};

static CHAR2TYPE_END: phf::Map<char, &'static str> = phf_map! {
    '*' => "</bold_314159265358979323846264338327950288419716939937510>",
    '/' => "</italic_314159265358979323846264338327950288419716939937510>",
    '_' => "</underline_314159265358979323846264338327950288419716939937510>",
    '+' => "</strikethrough_314159265358979323846264338327950288419716939937510>",
    '=' => "</verbatim_314159265358979323846264338327950288419716939937510>",
    '~' => "</code_314159265358979323846264338327950288419716939937510>"
};

static CHAR2TYPE_BEGIN_V2: phf::Map<char, char> = phf_map! {
    '*' => '\u{0001}',
    '/' => '\u{0003}',
    '_' => '\u{0005}',
    '+' => '\u{0007}',
    '=' => '\u{0009}',
    '~' => '\u{0011}',
};

static CHAR2TYPE_END_V2: phf::Map<char, char> = phf_map! {
    '*' => '\u{0002}',
    '/' => '\u{0004}',
    '_' => '\u{0006}',
    '+' => '\u{0008}',
    '=' => '\u{0010}',
    '~' => '\u{0012}',
};

#[derive(Debug, Clone, PartialEq)]
enum Token {
    Char(char),
    BoldStart,
    BoldEnd,
    ItalicStart,
    ItalicEnd,
    UnderlineStart,
    UnderlineEnd,
    CodeStart,
    CodeEnd,
    VerbatimStart,
    VerbatimEnd,
    StrikethroughStart,
    StrikethroughEnd,
}

/// use custom parse complex nested markup, which output Token Stream
impl Token {
    fn text(&self) -> char {
        match self {
            Token::BoldStart => '*',
            Token::BoldEnd => '*',
            Token::ItalicStart => '/',
            Token::ItalicEnd => '/',
            Token::UnderlineStart => '_',
            Token::UnderlineEnd => '_',
            Token::CodeStart => '~',
            Token::CodeEnd => '~',
            Token::VerbatimStart => '=',
            Token::VerbatimEnd => '=',
            Token::StrikethroughStart => '+',
            Token::StrikethroughEnd => '+',
            Token::Char(c) => *c,
        }
    }

    /// for test only
    fn xml(&self) -> String {
        match self {
            Token::BoldStart => CHAR2TYPE_BEGIN.get(&'*').unwrap().to_string(),
            Token::BoldEnd => CHAR2TYPE_END.get(&'*').unwrap().to_string(),
            Token::ItalicStart => CHAR2TYPE_BEGIN.get(&'/').unwrap().to_string(),
            Token::ItalicEnd => CHAR2TYPE_END.get(&'/').unwrap().to_string(),
            Token::UnderlineStart => CHAR2TYPE_BEGIN.get(&'_').unwrap().to_string(),
            Token::UnderlineEnd => CHAR2TYPE_END.get(&'_').unwrap().to_string(),
            Token::CodeStart => CHAR2TYPE_BEGIN.get(&'~').unwrap().to_string(),
            Token::CodeEnd => CHAR2TYPE_END.get(&'~').unwrap().to_string(),
            Token::VerbatimStart => CHAR2TYPE_BEGIN.get(&'=').unwrap().to_string(),
            Token::VerbatimEnd => CHAR2TYPE_END.get(&'=').unwrap().to_string(),
            Token::StrikethroughStart => CHAR2TYPE_BEGIN.get(&'+').unwrap().to_string(),
            Token::StrikethroughEnd => CHAR2TYPE_END.get(&'+').unwrap().to_string(),
            Token::Char(c) => (*c).to_string(),
        }
    }
}

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

/// Check whether the start marker is valid through: PRE MARKER FIRST_CHAR_OF_CONTENT
// i?
fn is_start_marker_valid(text: &str, i: usize, marker_stack: &MarkerStack) -> bool {
    // Hight Priority?
    match marker_stack.last() {
        Some((marker, _i)) if MARKER_SET_HIGH_PRIORITY.contains(marker) => {
            return false;
        }
        _ => {}
    }

    // PRE
    let pre_valid = if (i == 0)
        || (NORMAL_PRE.contains(&text.chars().nth(i - 1).unwrap()))
        || (marker_stack.contains(&(text.chars().nth(i - 1).unwrap(), i - 1)))
    {
        true
    } else {
        false
    };

    // marker
    let text_i = text.chars().nth(i).unwrap();
    let marker_valid = if MARKER_SET.contains(&text_i)
        && !marker_stack.history_state_without_last().contains(&text_i)
    {
        true
    } else {
        false
    };

    // first char of content
    let n = text.chars().count();
    let first_char_of_content_valid = if i + 1 >= n {
        false
    } else if WHITESPACE_SET.contains(&text.chars().nth(i + 1).unwrap()) {
        false
    } else {
        true
    };

    // println!(
    //     "pre_valid={}, marker_valid={}, first_char_of_content_valid={}",
    //     pre_valid, marker_valid, first_char_of_content_valid
    // );
    pre_valid && marker_valid && first_char_of_content_valid
}

fn is_end_marker_valid(text: &str, i: usize, marker_stack: &MarkerStack) -> bool {
    // println!("is_end_marker_valid ...");

    // last char of content
    let text_nm1 = text.chars().nth(i - 1).unwrap();
    // println!("{}", text_nm1);
    let last_of_char_content_valid = if !WHITESPACE_SET.contains(&text_nm1) {
        true
    } else {
        false
    };

    // println!("last_of_char_content_valid={}", last_of_char_content_valid);

    // marker
    let marker_valid = if marker_stack.len() > 0
        && text.chars().nth(i).unwrap() == marker_stack.last().unwrap().0
    {
        true
    } else {
        false
    };

    // println!("marker_valid={}", marker_valid);

    // post valid
    let post_valid = if i == text.chars().count() - 1 {
        true
    } else if END_OF_LINE_SET.contains(&text.chars().nth(i + 1).unwrap()) {
        true
    } else if NORMAL_POST.contains(&text.chars().nth(i + 1).unwrap()) {
        true
    } else if marker_stack
        .history_state_without_last()
        .contains(&text.chars().nth(i + 1).unwrap())
    {
        // lookahead
        let mut tmp_history_state_without_last = marker_stack.history_state_without_last().clone();
        let n_history_state = tmp_history_state_without_last.len();
        let mut post_valid = false;
        let mut flag_break = false;

        let n = text.chars().count();

        let mut last_j: usize = 0;

        for j in i + 1..(if i + 1 + n_history_state < n {
            i + 1 + n_history_state
        } else {
            n
        }) {
            last_j = j;
            if *tmp_history_state_without_last.last().unwrap() == text.chars().nth(j).unwrap() {
                tmp_history_state_without_last.pop();
            } else {
                flag_break = true;
                break;
            }
        }

        if flag_break && WHITESPACE_SET.contains(&text.chars().nth(last_j).unwrap()) {
            post_valid = true;
        } else {
            if last_j + 1 < n {
                if WHITESPACE_SET.contains(&text.chars().nth(last_j + 1).unwrap()) {
                    post_valid = true;
                } else {
                    post_valid = false;
                }
            } else {
                post_valid = true;
            }
        }

        post_valid
    } else {
        false
    };

    // println!(
    //     "last_of_char_content_valid={}, marker_valid={},  post_valid={}",
    //     last_of_char_content_valid, marker_valid, post_valid
    // );
    last_of_char_content_valid && marker_valid && post_valid
}

/// markup preprocessor: mark the bound of marker using xml style string
/// 仅当输入第一个char为markup开始时，才消费，且消费到第一个markup_end，含后续的空格。之后不再消费
///
/// - mark the bound of marker using xml style string
///   - `CHAR2TYPE_BEGIN`
///   - `CHAR2TYPE_END`
/// - map into Token Stream using Vec<Token>
///   - `Token`
pub(crate) fn text_markup_inner_preprocesser<'a>()
-> impl Parser<'a, &'a str, Vec<Token>, extra::Full<Rich<'a, char>, SimpleState<ParserState>, ()>>
+ Clone {
    custom(
        |inp: &mut InputRef<
            'a,
            '_,
            &'a str,
            extra::Full<Rich<'a, char>, SimpleState<ParserState>, ()>,
        >| {
            // println!("\ninner: cursor_inner={:?}", inp.cursor().inner());
            let text: &str = inp.slice_from(std::ops::RangeFrom {
                start: &inp.cursor(),
            });
            // println!("inner: text={:?}", text);

            let mut i: usize = 0;
            let n: usize = text.chars().count();

            let mut marker_stack = MarkerStack::default();
            let mut result = Result::default();

            // println!("n={}, i={}", n, i);
            while i < n {
                // println!("n={}, i={}", n, i);
                if i < n - 1 && is_start_marker_valid(text, i, &marker_stack) {
                    // println!("marker star");
                    let marker_char = text.chars().nth(i).unwrap();
                    marker_stack.push((marker_char, i));
                    result.push(i, CHAR2TYPE_BEGIN.get(&marker_char).unwrap().to_string());
                    i = i + 1;
                } else if i > 0 && is_end_marker_valid(text, i, &marker_stack) {
                    // println!("marker end");

                    if let Some((marker_char, _)) = marker_stack.pop() {
                        result.push(i, CHAR2TYPE_END.get(&marker_char).unwrap().to_string());
                        i = i + 1;
                    }
                } else {
                    i = i + 1;
                }
            }

            while marker_stack.len() > 0 {
                if let Some((_, i_input)) = marker_stack.pop() {
                    while result.len() > 0 && result.last().unwrap().0 >= i_input {
                        result.pop();
                    }
                }
            }

            let mut ans: Vec<Token> = vec![];
            // println!("inner result={:?}", result);
            // 仅当为marker开始时候，才计算ans; 否则， ans=[]
            if let Some(first_marker_symbol) = result.get(&(0 as usize)) {
                let first_marker_symbol_end = first_marker_symbol.replace("<", "</");
                for i in 0..n {
                    if let Some(c) = inp.next() {
                        if result.contains(&i) {
                            match result.get(&i) {
                                Some(x) if x == CHAR2TYPE_BEGIN.get(&'*').unwrap() => {
                                    ans.push(Token::BoldStart)
                                }
                                Some(x) if x == CHAR2TYPE_END.get(&'*').unwrap() => {
                                    ans.push(Token::BoldEnd);
                                    if *x == first_marker_symbol_end {
                                        // 结束第一个marker, 不再消耗
                                        break;
                                    }
                                }
                                Some(x) if x == CHAR2TYPE_BEGIN.get(&'/').unwrap() => {
                                    ans.push(Token::ItalicStart)
                                }
                                Some(x) if x == CHAR2TYPE_END.get(&'/').unwrap() => {
                                    ans.push(Token::ItalicEnd);
                                    if *x == first_marker_symbol_end {
                                        break;
                                    }
                                }
                                Some(x) if x == CHAR2TYPE_BEGIN.get(&'_').unwrap() => {
                                    ans.push(Token::UnderlineStart)
                                }
                                Some(x) if x == CHAR2TYPE_END.get(&'_').unwrap() => {
                                    ans.push(Token::UnderlineEnd);
                                    if *x == first_marker_symbol_end {
                                        break;
                                    }
                                }
                                Some(x) if x == CHAR2TYPE_BEGIN.get(&'+').unwrap() => {
                                    ans.push(Token::StrikethroughStart)
                                }
                                Some(x) if x == CHAR2TYPE_END.get(&'+').unwrap() => {
                                    ans.push(Token::StrikethroughEnd);
                                    if *x == first_marker_symbol_end {
                                        break;
                                    }
                                }
                                Some(x) if x == CHAR2TYPE_BEGIN.get(&'~').unwrap() => {
                                    ans.push(Token::CodeStart)
                                }
                                Some(x) if x == CHAR2TYPE_END.get(&'~').unwrap() => {
                                    ans.push(Token::CodeEnd);
                                    if *x == first_marker_symbol_end {
                                        break;
                                    }
                                }
                                Some(x) if x == CHAR2TYPE_BEGIN.get(&'=').unwrap() => {
                                    ans.push(Token::VerbatimStart)
                                }
                                Some(x) if x == CHAR2TYPE_END.get(&'=').unwrap() => {
                                    ans.push(Token::VerbatimEnd);
                                    if *x == first_marker_symbol_end {
                                        break;
                                    }
                                }
                                _ => {}
                            }
                        } else {
                            ans.push(Token::Char(c));
                        }
                    } else {
                        let error = Rich::custom::<&str>(
                            SimpleSpan::from(Range {
                                start: *inp.cursor().inner(),
                                end: *inp.cursor().inner() + i,
                            }),
                            &format!("text_markup_parser: has errorsxx"),
                        );

                        return Err(error);
                    }
                }

                while let Some(c) = inp.peek() {
                    if matches!(c, '\t' | ' ') {
                        inp.next();
                        ans.push(Token::Char(c));
                    } else {
                        break;
                    }
                }
            }

            // println!("inner: cursor_inner={:?}", inp.cursor().inner());
            // println!("inner: ans={:?}", ans);
            Ok(ans)
        },
    )
}

#[allow(unused)]
pub(crate) fn text_markup_inner_preprocesser_v2<'a>()
-> impl Parser<'a, &'a str, String, extra::Full<Rich<'a, char>, SimpleState<ParserState>, ()>> + Clone
{
    custom(
        |inp: &mut InputRef<
            'a,
            '_,
            &'a str,
            extra::Full<Rich<'a, char>, SimpleState<ParserState>, ()>,
        >| {
            println!("\ninner: cursor_inner={:?}", inp.cursor().inner());
            let text: &str = inp.slice_from(std::ops::RangeFrom {
                start: &inp.cursor(),
            });
            println!("inner: text={:?}", text);

            let mut i: usize = 0;
            let n: usize = text.chars().count();

            let mut marker_stack = MarkerStack::default();
            let mut result = Result::default();

            // println!("n={}, i={}", n, i);
            while i < n {
                // println!("n={}, i={}", n, i);
                if i < n - 1 && is_start_marker_valid(text, i, &marker_stack) {
                    // println!("marker star");
                    let marker_char = text.chars().nth(i).unwrap();
                    marker_stack.push((marker_char, i));
                    result.push(i, CHAR2TYPE_BEGIN_V2.get(&marker_char).unwrap().to_string());
                    i = i + 1;
                } else if i > 0 && is_end_marker_valid(text, i, &marker_stack) {
                    // println!("marker end");

                    if let Some((marker_char, _)) = marker_stack.pop() {
                        result.push(i, CHAR2TYPE_END_V2.get(&marker_char).unwrap().to_string());
                        i = i + 1;
                    }
                } else {
                    i = i + 1;
                }
            }

            while marker_stack.len() > 0 {
                if let Some((_, i_input)) = marker_stack.pop() {
                    while result.len() > 0 && result.last().unwrap().0 >= i_input {
                        result.pop();
                    }
                }
            }
            let mut ans: String = String::new();

            for (i, c) in text.chars().enumerate() {
                if result.contains(&i) {
                    match result.get(&i) {
                        Some(x) => ans.push_str(x),
                        _ => {}
                    }
                } else {
                    ans.push(c);
                }
            }

            // for i in 0..n {
            //     if let Some(c) = inp.next() {
            //         if result.contains(&i) {
            //             match result.get(&i) {
            //                 Some(x) => ans.push_str(x),
            //                 _ => {}
            //             }
            //         } else {
            //             ans.push(c);
            //         }
            //     }
            // }

            println!("    result={:?}", result);
            println!("  inner: cursor_inner={:?}", inp.cursor().inner());
            println!("  text_markup_inner_preprocesser: ans={:?}", ans);
            Ok(ans)
            // let a = ans.into_boxed_str();
            // let b = Box::leak(a);
            // Ok(b)
            // Ok(ans.as_str())
        },
    )
}

/// markup parser: MARKER MARKER
pub(crate) fn text_markup_outer_parser<'a>() -> impl Parser<
    'a,
    &'a [Token],
    NodeOrToken<GreenNode, GreenToken>,
    extra::Full<Rich<'a, Token>, SimpleState<ParserState>, ()>,
> + Clone {
    recursive(|markup| {
        let base_case = any()
            .filter(|c: &Token| matches!(c, Token::Char(_)))
            .repeated()
            .at_least(1)
            .collect::<Vec<Token>>()
            .map(|s| {
                // let mut children = vec![];

                NodeOrToken::<GreenNode, GreenToken>::Token(GreenToken::new(
                    OrgSyntaxKind::Text.into(),
                    &s.iter().map(|s| s.text()).collect::<String>(),
                ))

                // let _token = NodeOrToken::<GreenNode, GreenToken>::Token(GreenToken::new(
                //     OrgSyntaxKind::Text.into(),
                //     &s.iter().map(|s| s.text()).collect::<String>(),
                // ));
                // children.push(_token);

                // NodeOrToken::<GreenNode, GreenToken>::Node(GreenNode::new(
                //     OrgSyntaxKind::Text.into(),
                //     children,
                // ))
            });

        let get_mark_parser = |start: Token, end: Token| {
            just(start)
                // .map(|s| {
                //     println!("outer: s0={:?}", s);
                //     s
                // })
                .then(
                    markup
                        .clone()
                        .repeated()
                        .at_least(1)
                        .collect::<Vec<NodeOrToken<GreenNode, GreenToken>>>(),
                )
                // .map(|s| {
                //     println!("outer: s1={:?}", s);
                //     s
                // })
                .then(just(end))
                // .map(|s| {
                //     println!("outer: s2={:?}", s);
                //     s
                // })
                .then(
                    one_of([Token::Char(' '), Token::Char('\t')])
                        .repeated()
                        .collect::<Vec<Token>>(),
                )
                // .map(|s| {
                //     println!("outer: s3={:?}", s);
                //     s
                // })
                .map(|(((start_marker, content), end_marker), whitespaces)| {
                    let marker_syntax_kind = match start_marker {
                        Token::BoldStart => OrgSyntaxKind::Asterisk,
                        Token::ItalicStart => OrgSyntaxKind::Slash,
                        Token::UnderlineStart => OrgSyntaxKind::UnderScore,
                        Token::StrikethroughStart => OrgSyntaxKind::Plus,
                        Token::VerbatimStart => OrgSyntaxKind::Equals,
                        Token::CodeStart => OrgSyntaxKind::Tilde,
                        _ => OrgSyntaxKind::At,
                    };

                    let node_syntax_kind = match start_marker {
                        Token::BoldStart => OrgSyntaxKind::Bold,
                        Token::ItalicStart => OrgSyntaxKind::Italic,
                        Token::UnderlineStart => OrgSyntaxKind::UnderLine,
                        Token::StrikethroughStart => OrgSyntaxKind::StrikeThrough,
                        Token::VerbatimStart => OrgSyntaxKind::Verbatim,
                        Token::CodeStart => OrgSyntaxKind::Code,
                        _ => OrgSyntaxKind::At,
                    };

                    let mut children = vec![];
                    children.push(NodeOrToken::Token(GreenToken::new(
                        marker_syntax_kind.into(),
                        &start_marker.text().to_string(),
                    )));

                    for n in content {
                        children.push(n);
                    }

                    children.push(NodeOrToken::Token(GreenToken::new(
                        marker_syntax_kind.into(),
                        &end_marker.text().to_string(),
                    )));

                    if whitespaces.len() > 0 {
                        let mut ws = String::new();
                        for w in whitespaces {
                            ws.push_str(&w.text().to_string())
                        }

                        children.push(NodeOrToken::Token(GreenToken::new(
                            OrgSyntaxKind::Whitespace.into(),
                            &ws,
                        )));
                    }
                    let ans = NodeOrToken::Node(GreenNode::new(node_syntax_kind.into(), children));
                    // println!("outer: s4={:?}\n", ans);
                    ans
                })
        };

        let bold = get_mark_parser(Token::BoldStart, Token::BoldEnd);
        let italic = get_mark_parser(Token::ItalicStart, Token::ItalicEnd);
        let underline = get_mark_parser(Token::UnderlineStart, Token::UnderlineEnd);
        let strikethrough = get_mark_parser(Token::StrikethroughStart, Token::StrikethroughEnd);
        let verbatim = get_mark_parser(Token::VerbatimStart, Token::VerbatimEnd);
        let code = get_mark_parser(Token::CodeStart, Token::CodeEnd);

        bold.or(italic)
            .or(underline)
            .or(strikethrough)
            .or(verbatim)
            .or(code)
            .or(base_case)
    })
}

/// markup parser: MARKER MARKER
pub(crate) fn text_markup_outer_parser_v2<'a>() -> impl Parser<
    'a,
    &'a str,
    NodeOrToken<GreenNode, GreenToken>,
    extra::Full<Rich<'a, char>, SimpleState<ParserState>, ()>,
> + Clone {
    recursive(|markup| {
        let base_case = any()
            .filter(|c: &char| {
                ![
                    '\u{0001}', '\u{0002}', '\u{0003}', '\u{0004}', '\u{0005}', '\u{0006}',
                    '\u{0007}', '\u{0008}', '\u{0009}', '\u{0010}', '\u{0011}', '\u{0012}',
                ]
                .contains(c)
            })
            .repeated()
            .at_least(1)
            .collect::<String>()
            .map(|s| {
                println!("  dbg: s0={:?}", s);
                s
            })
            .map(|s| {
                let mut children = vec![];
                let _token = NodeOrToken::<GreenNode, GreenToken>::Token(GreenToken::new(
                    OrgSyntaxKind::Text.into(),
                    &s,
                ));
                children.push(_token);

                NodeOrToken::<GreenNode, GreenToken>::Node(GreenNode::new(
                    OrgSyntaxKind::Text.into(),
                    children,
                ))
            });

        let get_mark_parser = |start: char, end: char| {
            just(start)
                .map(|s| {
                    println!("  dbg: s0={:?}", s);
                    s
                })
                .then(
                    markup
                        .clone()
                        .repeated()
                        .at_least(1)
                        .collect::<Vec<NodeOrToken<GreenNode, GreenToken>>>(),
                )
                .map(|s| {
                    println!("  dbg: s1={:?}", s);
                    s
                })
                .then(just(end))
                .map(|s| {
                    println!("  dbg: s2={:?}", s);
                    s
                })
                .then(one_of(" \t").repeated().collect::<String>())
                .map(|(((start_marker, content), end_marker), whitespaces)| {
                    let marker_syntax_kind = match start_marker {
                        '\u{0001}' => OrgSyntaxKind::Asterisk,
                        '\u{0003}' => OrgSyntaxKind::Slash,
                        '\u{0005}' => OrgSyntaxKind::UnderScore,
                        '\u{0007}' => OrgSyntaxKind::Plus,
                        '\u{0009}' => OrgSyntaxKind::Equals,
                        '\u{0011}' => OrgSyntaxKind::Tilde,
                        _ => OrgSyntaxKind::At,
                    };

                    let node_syntax_kind = match start_marker {
                        '\u{0001}' => OrgSyntaxKind::Bold,
                        '\u{0003}' => OrgSyntaxKind::Italic,
                        '\u{0005}' => OrgSyntaxKind::UnderLine,
                        '\u{0007}' => OrgSyntaxKind::StrikeThrough,
                        '\u{0009}' => OrgSyntaxKind::Verbatim,
                        '\u{0011}' => OrgSyntaxKind::Code,
                        _ => OrgSyntaxKind::At,
                    };

                    let mut children = vec![];
                    children.push(NodeOrToken::Token(GreenToken::new(
                        marker_syntax_kind.into(),
                        match start_marker {
                            '\u{0001}' => "*",
                            '\u{0003}' => "/",
                            '\u{0005}' => "_",
                            '\u{0007}' => "+",
                            '\u{0009}' => "=",
                            '\u{0011}' => "~",
                            _ => "",
                        },
                    )));

                    for n in content.clone() {
                        children.push(n);
                    }

                    children.push(NodeOrToken::Token(GreenToken::new(
                        marker_syntax_kind.into(),
                        match end_marker {
                            '\u{0002}' => "*",
                            '\u{0004}' => "/",
                            '\u{0006}' => "_",
                            '\u{0008}' => "+",
                            '\u{0010}' => "=",
                            '\u{0012}' => "~",
                            _ => "",
                        },
                    )));

                    if whitespaces.len() > 0 {
                        let mut ws = String::new();
                        for w in whitespaces.chars() {
                            ws.push(w)
                        }

                        children.push(NodeOrToken::Token(GreenToken::new(
                            OrgSyntaxKind::Whitespace.into(),
                            &ws,
                        )));
                    }

                    println!(
                        "get_mark_parser: {:?}, {:?}, {:?}",
                        start_marker, content, end_marker
                    );

                    NodeOrToken::Node(GreenNode::new(node_syntax_kind.into(), children))
                })
        };

        let bold = get_mark_parser('\u{0001}', '\u{0002}');
        let italic = get_mark_parser('\u{0003}', '\u{0004}');
        let underline = get_mark_parser('\u{0005}', '\u{0006}');
        let strikethrough = get_mark_parser('\u{0007}', '\u{0008}');
        let verbatim = get_mark_parser('\u{0009}', '\u{0010}');
        let code = get_mark_parser('\u{0011}', '\u{0012}');

        bold.or(italic)
            .or(underline)
            .or(strikethrough)
            .or(verbatim)
            .or(code)
            .or(base_case)
    })
}

/// Note: we can't use nested, since &str -parser1-> &[Token] -parser2-> Node
/// full_markup_parser
#[allow(unused)]
pub(crate) fn text_markup_parser_todo<'a>()
-> impl Parser<'a, &'a str, S2, extra::Full<Rich<'a, char>, SimpleState<ParserState>, ()>> + Clone {
    text_markup_inner_preprocesser().map(|tokens: Vec<Token>| {
        let p = text_markup_outer_parser();

        S2::Single(p.parse(&tokens[..]).into_result().unwrap())
    })
}

pub(crate) fn demo<'a>()
-> impl Parser<'a, &'a str, S2, extra::Full<Rich<'a, char>, SimpleState<ParserState>, ()>> + Clone {
    one_of("*/")
        .then(none_of("*/").repeated().collect::<String>())
        .then(one_of("*/"))
        .map_with(|((m1, c), m2): ((char, _), char), e| {
            println!("m1={}, c={}, m2={}, e={:?}", m1, c, m2, e.span());
            let mut children = vec![];
            children.push(NodeOrToken::Token(GreenToken::new(
                OrgSyntaxKind::Text.into(),
                &m1.to_string(),
            )));
            children.push(NodeOrToken::Token(GreenToken::new(
                OrgSyntaxKind::Text.into(),
                &m2.to_string(),
            )));
            children.push(NodeOrToken::Token(GreenToken::new(
                OrgSyntaxKind::Text.into(),
                &c,
            )));
            S2::Single(NodeOrToken::Node(GreenNode::new(
                OrgSyntaxKind::Bold.into(),
                children,
            )))
        })
}

/// text_markup_parser:
/// 两个parser的类型不一致，无法用nested_in, 需要手动嵌套用inner_parser.try_map(|s| outer_parser.parse)解析
pub(crate) fn text_markup_parser<'a>() -> impl Parser<
    'a,
    &'a str,
    S2,
    // NodeOrToken<GreenNode, GreenToken>,
    extra::Full<Rich<'a, char>, SimpleState<ParserState>, ()>,
> + Clone {
    // text_markup_inner_preprocesser().map(|tokens: Vec<Token>| {
    //     let p = text_markup_outer_parser();
    //     p.parse(&tokens[..]).into_result().unwrap()
    // })

    text_markup_inner_preprocesser()
        // .map(|s| {
        //     println!("s={:?}", s);
        //     s
        // })
        .try_map_with(|tokens: Vec<Token>, e| {
            // println!("tokens={:?}", tokens);

            let p = text_markup_outer_parser();

            let a = p
                // .lazy()
                .parse(&tokens[..]);
            if a.has_output() {
                // println!("  text_markup_parser's output={:?}", a);
                // Ok(a.into_result().unwrap())
                Ok(S2::Single(a.into_result().unwrap()))
            } else {
                let error = Rich::custom::<&str>(
                    SimpleSpan::from(Range {
                        start: e.span().start(),
                        end: e.span().end(),
                    }),
                    &format!("text_markup_parser: has errorsxx"),
                );
                // // debug
                // for (i, e) in a.errors().enumerate() {
                //     println!("text_markup_parsers' error{:?}={:?}", i, e);
                // }

                // emitter.emit(error);
                Err(error)
            }
        })
}

// to_slice: 会 恢复为preprocess之前的str
#[allow(unused)]
pub(crate) fn text_markup_parser_v2<'a>() -> impl Parser<
    'a,
    &'a str,
    S2,
    // NodeOrToken<GreenNode, GreenToken>,
    extra::Full<Rich<'a, char>, SimpleState<ParserState>, ()>,
> + Clone {
    // text_markup_inner_preprocesser().map(|tokens: Vec<Token>| {
    //     let p = text_markup_outer_parser();
    //     p.parse(&tokens[..]).into_result().unwrap()
    // })

    text_markup_outer_parser_v2()
        // .lazy( )
        .nested_in(
            text_markup_inner_preprocesser_v2()
                // .lazy()
                .to_slice(),
        )
        .validate(|result, e, emitter| {
            println!("  validate: result={:?}, e={:?}", result, e.span());
            result
        })
        // .lazy()
        .map(|s| S2::Single(s))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::section::section_unknown_parser;
    use crate::parser::syntax::OrgLanguage;

    use rowan::SyntaxNode;

    #[test]
    fn test_inner_preprocessor() {
        let inputs = vec![
            (
                "*/_bold-italic-underline_/*",
                "<bold><italic><underline>bold-italic-underline</underline></italic></bold>",
            ),
            ("~=*_/inner-most/_*=~ ", "<code>=*_/inner-most/_*=</code> "),
            // bui: 3!
            (
                "*/_bold-italic-underline_/*",
                "<bold><italic><underline>bold-italic-underline</underline></italic></bold>",
            ),
            (
                "*_/bold-underline-italic/_*",
                "<bold><underline><italic>bold-underline-italic</italic></underline></bold>",
            ),
            (
                "_/*underline-italic-bold*/_",
                "<underline><italic><bold>underline-italic-bold</bold></italic></underline>",
            ),
            (
                "_*/underline-bold-italic/*_",
                "<underline><bold><italic>underline-bold-italic</italic></bold></underline>",
            ),
            (
                "/_*italic-underline-bold*_/",
                "<italic><underline><bold>italic-underline-bold</bold></underline></italic>",
            ),
            (
                "/*_italic-bold-underline_*/",
                "<italic><bold><underline>italic-bold-underline</underline></bold></italic>",
            ),
            // +bui: 2
            (
                "+/*_strikethrough-italic-bold-underline_*/+",
                "<strikethrough><italic><bold><underline>strikethrough-italic-bold-underline</underline></bold></italic></strikethrough>",
            ),
            (
                "+/_*strikethrough-italic-underline-bold*_/+",
                "<strikethrough><italic><underline><bold>strikethrough-italic-underline-bold</bold></underline></italic></strikethrough>",
            ),
            // high priority
            (
                "*_~inner-most~_*",
                "<bold><underline><code>inner-most</code></underline></bold>",
            ),
            (
                "*_~=inner-most=~_*",
                "<bold><underline><code>=inner-most=</code></underline></bold>",
            ),
            (
                "*_=~inner-most~=_*",
                "<bold><underline><verbatim>~inner-most~</verbatim></underline></bold>",
            ),
            ("~=*_/inner-most/_*=~", "<code>=*_/inner-most/_*=</code>"),
            (
                "=~*_/inner-most/_*~=",
                "<verbatim>~*_/inner-most/_*~</verbatim>",
            ),
        ];

        for (i, (input, answer)) in inputs.iter().enumerate() {
            let mut state = SimpleState(ParserState::default());
            let preprocessor = text_markup_inner_preprocesser();
            let t = preprocessor.parse_with_state(input, &mut state);

            if t.has_errors() {
                for e in t.errors() {
                    println!("inner_preprocessor_error(parse_with_state) = {:?}", e);
                }
            }
            assert_eq!(t.has_output(), true);

            let output = t.into_result().unwrap();
            let output = output.iter().fold(String::new(), |mut acc, e| {
                acc.push_str(&e.xml());
                acc
            });
            let output = output.replace("_314159265358979323846264338327950288419716939937510", "");
            println!("output={:?}", output);

            assert_eq!(&output, answer);
        }
    }

    //     #[test]
    //     fn test_text_markup_parser() {
    //         let input = "*/_+=all=+_/*";
    //         let parser = text_markup_parser();
    //         let node = parser.parse(input).unwrap();
    //         println!("node={:?}", node);
    //         let syntax_tree: SyntaxNode<OrgLanguage> =
    //             SyntaxNode::new_root(node.into_node().expect("xxx"));

    //         let answer = r##"Bold@0..13
    //   Asterisk@0..1 "*"
    //   Italic@1..12
    //     Slash@1..2 "/"
    //     UnderLine@2..11
    //       UnderScore@2..3 "_"
    //       StrikeThrough@3..10
    //         Plus@3..4 "+"
    //         Verbatim@4..9
    //           Equals@4..5 "="
    //           Text@5..8
    //             Text@5..8 "all"
    //           Equals@8..9 "="
    //         Plus@9..10 "+"
    //       UnderScore@10..11 "_"
    //     Slash@11..12 "/"
    //   Asterisk@12..13 "*"
    // "##;
    //         assert_eq!(format!("{:#?}", syntax_tree), answer);
    //     }

    #[test]
    fn test_text_markup_rpt_parser() {
        // all normal OK
        // all bold OK
        // a /it/ bad: bad

        let input = "a /it/ line";
        // let input = "/it/ *bo*";
        // let input = "\u{0001}it\u{0002} \u{0003}it\u{0004}  \t";
        // let input = "\u{0001}it\u{0002} ";

        let parser = text_markup_parser()
            // text_markup_parser_v2()
            .or(text_parser())
            // text_markup_parser().or(text_parser())
            // text_parser()
            // demo()
            // .or(text_markup_parser())
            .repeated()
            .at_least(1)
            .collect::<Vec<_>>();

        let nodes = parser.parse(input);
        for e in nodes.errors() {
            println!("error={:?}", e);
        }
        println!("test_text_markup_rpt_parser: nodes={:?}\n\n", nodes);

        for _node in nodes.into_result().unwrap() {
            match _node {
                S2::Single(node) => match node {
                    NodeOrToken::Token(t) => {
                        println!(" token={}", t);
                    }
                    NodeOrToken::Node(n) => {
                        let syntax_tree: SyntaxNode<OrgLanguage> = SyntaxNode::new_root(n);
                        println!("  node={:#?}", syntax_tree);
                    }
                },
                _ => {}
            }
        }
    }
}
