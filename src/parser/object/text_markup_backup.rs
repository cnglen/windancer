//! Text markup parser, including bold, italic, underline, strikethrough, verbatim and code.
use crate::parser::ParserState;
use crate::parser::S2;
// use crate::parser::object::text_parser;
use crate::parser::syntax::OrgSyntaxKind;
use chumsky::input::InputRef;
use chumsky::inspector::SimpleState;
use chumsky::prelude::*;
use phf::{phf_map, phf_set};
use rowan::{GreenNode, GreenToken, NodeOrToken};
use std::collections::HashMap;
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
//
//  - text[i-1] ~ PRE
//  - text[i]   ~ start_MARKER
//  - text[i+1] ~ FIRST_CHAR_OF_CONTENT
//
fn is_start_marker_valid(text: &str, i: usize, marker_stack: &MarkerStack) -> bool {
    // if in hight priority marker(=~), disabled other marker
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
    // text[i] in MARKER_SET and text[i] not a END marker
    // why without_last?
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

// text[i] ~ MARKER
fn is_end_marker_valid(text: &str, i: usize, marker_stack: &MarkerStack) -> bool {
    // CONTENTS may not end with whitespace
    let text_nm1 = text.chars().nth(i - 1).unwrap(); // last char of content
    let last_of_char_content_valid = if !WHITESPACE_SET.contains(&text_nm1) {
        true
    } else {
        false
    };
    // println!("is_end_marker_valid: last_char of conent=<{}>", text_nm1);
    // println!("last_of_char_content_valid={}", last_of_char_content_valid);

    // MARKER: match the last marker of marker_statck
    let marker_valid = if marker_stack.len() > 0
        && text.chars().nth(i).unwrap() == marker_stack.last().unwrap().0
    {
        true
    } else {
        false
    };

    // println!("marker_stack_last={:?}, text[i]={:?}, marker_valid={}", marker_stack.last(), text.chars().nth(i), marker_valid);

    // POST ~ text[i+1]
    //  - in NORMAL_POST := a whitespace character, -, ., ,, ;, :, !, ?, ', ), }, [, ", \ (backslash)
    //  - the end of a line := \r\n
    //  - 后面跟state中的某个marker, 且为history_state的逆序，且后续的字符为whitespace或eol或eof
    let post_valid = if i == text.chars().count() - 1 {
        // end of file
        true
    } else if END_OF_LINE_SET.contains(&text.chars().nth(i + 1).unwrap()) {
        // end of a line
        true
    } else if NORMAL_POST.contains(&text.chars().nth(i + 1).unwrap()) {
        //
        true
    } else if marker_stack // *_/xx/_*
        .history_state_without_last()
        .contains(&text.chars().nth(i + 1).unwrap())
    {
        // lookahead
        let mut tmp_history_state_without_last = marker_stack.history_state_without_last().clone();
        let n_history_state = tmp_history_state_without_last.len();
        let post_valid: bool;
        let mut flag_break = false;

        let n = text.chars().count();

        let mut last_j: usize = 0;

        let range_end = if i + 1 + n_history_state < n {
            i + 1 + n_history_state
        } else {
            n
        };
        // println!(
        //     "range_end={range_end}, tmp_history_state_without_last={:?}",
        //     tmp_history_state_without_last
        // );
        for j in i + 1..range_end {
            last_j = j;
            if *tmp_history_state_without_last.last().unwrap() == text.chars().nth(j).unwrap() {
                tmp_history_state_without_last.pop();
            } else {
                flag_break = true;
                break;
            }
        }

        let text_lastj = text.chars().nth(last_j).unwrap();
        // println!("text_last_j={text_lastj:?}");
        // println!("{} VS {n}", last_j + 1);
        if flag_break
            && (WHITESPACE_SET.contains(&text_lastj) || END_OF_LINE_SET.contains(&text_lastj))
        {
            post_valid = true;
        } else {
            if last_j + 1 < n {
                let text_lastj_plus1 = text.chars().nth(last_j + 1).unwrap();
                if WHITESPACE_SET.contains(&text_lastj_plus1)
                    || END_OF_LINE_SET.contains(&text_lastj_plus1)
                {
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

    //   println!(
    //       r##"{}:{}:is_end_marker_valid(): text={text:?}, i={i}, text[i]={:?}, marker_stack={:?},
    // last_of_char_content_valid={}
    // marker_valid              ={}
    // post_valid                ={}"##,
    //       file!(),
    //       line!(),
    //       text.chars().nth(i).unwrap(),
    //       marker_stack.data,
    //       last_of_char_content_valid,
    //       marker_valid,
    //       post_valid
    //   );

    last_of_char_content_valid && marker_valid && post_valid
}

/// markup preprocessor: mark the bound of marker using xml style string
/// 仅当输入第一个char为markup开始时，才消费，且消费到第一个markup_end，含后续的空格和换行符。之后不再消费
///
/// - mark the bound of marker using xml style string
///   - `CHAR2TYPE_BEGIN`
///   - `CHAR2TYPE_END`
/// - map into Token Stream using Vec<Token>
///   - `Token`
fn text_markup_inner_preprocesser<'a>()
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
            // println!("inner_preprocessor: text={:?}", text);

            let mut i: usize = 0;
            let n: usize = text.chars().count();

            let mut marker_stack = MarkerStack::default();
            let mut result = Result::default();

            // println!("n={}, i={}", n, i);
            while i < n {
                // println!("n={}, i={}", n, i);
                // println!("text={text:?}, i={i}, {:?}", marker_stack.data);
                if i < n - 1 && is_start_marker_valid(text, i, &marker_stack) {
                    // println!("marker starts");
                    let marker_char = text.chars().nth(i).unwrap();
                    marker_stack.push((marker_char, i));
                    result.push(i, CHAR2TYPE_BEGIN.get(&marker_char).unwrap().to_string());
                    i = i + 1;
                } else if i > 0 && is_end_marker_valid(text, i, &marker_stack) {
                    // println!("marker ends");

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
                    if matches!(c, '\t' | ' ' | '\r' | '\n') {
                        inp.next();
                        ans.push(Token::Char(c));
                    } else {
                        break;
                    }
                }
            }

            // println!("inner: cursor_inner={:?}", inp.cursor().inner());
            // println!("inner_preprocessor: ans ={:?}", ans);
            Ok(ans)
        },
    )
}

/// markup parser: MARKER MARKER
pub(crate) fn text_markup_outer_parser<'a>(
) -> impl Parser<
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
                NodeOrToken::<GreenNode, GreenToken>::Token(GreenToken::new(
                    OrgSyntaxKind::Text.into(),
                    &s.iter().map(|s| s.text()).collect::<String>(),
                ))
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
                    one_of([
                        Token::Char(' '),
                        Token::Char('\t'),
                        Token::Char('\n'),
                        Token::Char('\r'),
                    ])
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

/// text_markup_parser:
/// Note: we can't use nested, since &str -parser1-> &[Token] -parser2-> Node
/// 两个parser的类型不一致，无法用nested_in, 需要手动嵌套用inner_parser.try_map(|s| outer_parser.parse)解析
pub(crate) fn text_markup_parser<'a>(
    object_parser: impl Parser<
        'a,
        &'a str,
        S2,
        extra::Full<Rich<'a, char>, SimpleState<ParserState>, ()>,
    > + Clone,
) -> impl Parser<
    'a,
    &'a str,
    S2,
    // NodeOrToken<GreenNode, GreenToken>,
    extra::Full<Rich<'a, char>, SimpleState<ParserState>, ()>,
> + Clone {
    text_markup_inner_preprocesser()
        .try_map_with(move |tokens: Vec<Token>, e| {
            // let p = text_markup_outer_parser(object_parser.clone());
            let p = text_markup_outer_parser();            
            let a = p.parse(&tokens);
            if a.has_output() {
                Ok(S2::Single(a.into_result().unwrap()))
            } else {
                let error = Rich::custom(e.span(), format!("text_markup_parser: has errorsxx"));
                Err(error)
            }
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::common::{get_parser_output, get_parsers_output};
    use crate::parser::object;
    use pretty_assertions::assert_eq; // 该包仅能用于测试

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
            (
                "*/bold-italic-underline/*\n",
                "<bold><italic>bold-italic-underline</italic></bold>\n",
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
            assert_eq!(&output, answer);
        }
    }

    #[test]
    fn test_01_nested_markup() {
        assert_eq!(
            get_parsers_output(object::objects_parser(), "*/_+=all=+_/*"),
            r##"Root@0..13
  Bold@0..13
    Asterisk@0..1 "*"
    Italic@1..12
      Slash@1..2 "/"
      UnderLine@2..11
        UnderScore@2..3 "_"
        StrikeThrough@3..10
          Plus@3..4 "+"
          Verbatim@4..9
            Equals@4..5 "="
            Text@5..8 "all"
            Equals@8..9 "="
          Plus@9..10 "+"
        UnderScore@10..11 "_"
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
    fn test_03_bad_nested_markup() {
        assert_eq!(
            get_parsers_output(
                object::objects_parser(),
                "_underline_ */_underline_ italic/"
            ),
            r##"Root@0..33
  UnderLine@0..12
    UnderScore@0..1 "_"
    Text@1..10 "underline"
    UnderScore@10..11 "_"
    Whitespace@11..12 " "
  Text@12..13 "*"
  Italic@13..33
    Slash@13..14 "/"
    UnderLine@14..26
      UnderScore@14..15 "_"
      Text@15..24 "underline"
      UnderScore@24..25 "_"
      Whitespace@25..26 " "
    Text@26..32 "italic"
    Slash@32..33 "/"
"##
        );
    }
}
