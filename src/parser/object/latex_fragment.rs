//! latex fragment parser
use crate::parser::ParserState;
use crate::parser::S2;
use crate::parser::object::entity::ENTITYNAME_TO_HTML;
use crate::parser::syntax::OrgSyntaxKind;
use chumsky::inspector::SimpleState;
use chumsky::prelude::*;
use rowan::{GreenNode, GreenToken, NodeOrToken};

// Latex Frament parser
pub(crate) fn latex_fragment_parser<'a>()
-> impl Parser<'a, &'a str, S2, extra::Full<Rich<'a, char>, SimpleState<ParserState>, ()>> + Clone {
    let pre = any::<_, extra::Full<Rich<'_, char>, SimpleState<ParserState>, ()>>()
        .filter(|c| !matches!(c, '$'));
    let border1 = none_of("\r\n \t.,;$");
    let border2 = none_of("\r\n \t.,$");
    let post =
        any().filter(|c: &char| c.is_ascii_punctuation() || matches!(c, ' ' | '\t' | '\r' | '\n'));

    let name = any()
        .filter(|c: &char| c.is_alphabetic())
        .repeated()
        .at_least(1)
        .collect::<String>()
        .filter(|name| !ENTITYNAME_TO_HTML.contains_key(name));

    // \(CONTENTS\)
    let t1 = just::<_, _, extra::Full<Rich<'_, char>, SimpleState<ParserState>, ()>>(r##"\("##)
        .then(
            // take_until
            any()
                .and_is(just(r##"\)"##).not())
                .repeated()
                .collect::<String>(),
        )
        .then(just(r##"\)"##))
        .map_with(|((dd_pre, content), dd_post), e| {
            e.state().prev_char = dd_post.chars().last();

            let mut children = vec![];
            children.push(NodeOrToken::Token(GreenToken::new(
                OrgSyntaxKind::BackSlash.into(),
                dd_pre
                    .chars()
                    .nth(0)
                    .expect("first char")
                    .to_string()
                    .as_str(),
            )));

            children.push(NodeOrToken::Token(GreenToken::new(
                OrgSyntaxKind::LeftRoundBracket.into(),
                dd_pre
                    .chars()
                    .nth(1)
                    .expect("second char")
                    .to_string()
                    .as_str(),
            )));

            children.push(NodeOrToken::Token(GreenToken::new(
                OrgSyntaxKind::Text.into(),
                &content,
            )));

            children.push(NodeOrToken::Token(GreenToken::new(
                OrgSyntaxKind::BackSlash.into(),
                dd_post
                    .chars()
                    .nth(0)
                    .expect("first_char")
                    .to_string()
                    .as_str(),
            )));

            children.push(NodeOrToken::Token(GreenToken::new(
                OrgSyntaxKind::RightRoundBracket.into(),
                dd_post
                    .chars()
                    .nth(1)
                    .expect("second char")
                    .to_string()
                    .as_str(),
            )));

            S2::Single(NodeOrToken::Node(GreenNode::new(
                OrgSyntaxKind::LatexFragment.into(),
                children,
            )))
        });

    // \[CONTENTS\]
    let t2 = just::<_, _, extra::Full<Rich<'_, char>, SimpleState<ParserState>, ()>>(r##"\["##)
        .then(
            // take_until
            any()
                .and_is(just(r##"\]"##).not())
                .repeated()
                .collect::<String>(),
        )
        .then(just(r##"\]"##))
        .map_with(|((dd_pre, content), dd_post), e| {
            e.state().prev_char = dd_post.chars().last();

            let mut children = vec![];
            children.push(NodeOrToken::Token(GreenToken::new(
                OrgSyntaxKind::BackSlash.into(),
                dd_pre
                    .chars()
                    .nth(0)
                    .expect("first char")
                    .to_string()
                    .as_str(),
            )));

            children.push(NodeOrToken::Token(GreenToken::new(
                OrgSyntaxKind::LeftSquareBracket.into(),
                dd_pre
                    .chars()
                    .nth(1)
                    .expect("second char")
                    .to_string()
                    .as_str(),
            )));

            children.push(NodeOrToken::Token(GreenToken::new(
                OrgSyntaxKind::Text.into(),
                &content,
            )));

            children.push(NodeOrToken::Token(GreenToken::new(
                OrgSyntaxKind::BackSlash.into(),
                dd_post
                    .chars()
                    .nth(0)
                    .expect("first_char")
                    .to_string()
                    .as_str(),
            )));

            children.push(NodeOrToken::Token(GreenToken::new(
                OrgSyntaxKind::RightSquareBracket.into(),
                dd_post
                    .chars()
                    .nth(1)
                    .expect("second char")
                    .to_string()
                    .as_str(),
            )));

            S2::Single(NodeOrToken::Node(GreenNode::new(
                OrgSyntaxKind::LatexFragment.into(),
                children,
            )))
        });

    // $$CONTENTS$$
    let t3 = just::<_, _, extra::Full<Rich<'_, char>, SimpleState<ParserState>, ()>>("$$")
        .then(
            // take_until
            any()
                .and_is(just("$$").not())
                .repeated()
                .collect::<String>(),
        )
        .then(just("$$"))
        .map_with(|((dd_pre, content), dd_post), e| {
            e.state().prev_char = dd_post.chars().last();
            let mut children = vec![];
            children.push(NodeOrToken::Token(GreenToken::new(
                OrgSyntaxKind::Dollar2.into(),
                dd_pre,
            )));

            children.push(NodeOrToken::Token(GreenToken::new(
                OrgSyntaxKind::Text.into(),
                &content,
            )));

            children.push(NodeOrToken::Token(GreenToken::new(
                OrgSyntaxKind::Dollar2.into(),
                dd_post,
            )));

            S2::Single(NodeOrToken::Node(GreenNode::new(
                OrgSyntaxKind::LatexFragment.into(),
                children,
            )))
        });

    // PRE$CHAR$POST
    let t4 = pre
        .then(just("$"))
        .then(none_of(".,?;\" \t"))
        .then(just("$"))
        .then_ignore(post.rewind())
        .map_with(|(((pre, d_pre), c), d_post), e| {
            e.state().prev_char = d_post.chars().last();

            let mut children = vec![];

            children.push(NodeOrToken::Token(GreenToken::new(
                OrgSyntaxKind::Dollar.into(),
                d_pre,
            )));

            children.push(NodeOrToken::Token(GreenToken::new(
                OrgSyntaxKind::Text.into(),
                &format!("{}", c),
            )));

            children.push(NodeOrToken::Token(GreenToken::new(
                OrgSyntaxKind::Dollar.into(),
                d_post,
            )));

            S2::Double(
                NodeOrToken::Token(GreenToken::new(
                    OrgSyntaxKind::Text.into(),
                    pre.to_string().as_str(),
                )),
                NodeOrToken::Node(GreenNode::new(
                    OrgSyntaxKind::LatexFragment.into(),
                    children,
                )),
            )
        });

    // PRE$BORDER1 BODY BORDER2$POST
    let t5 = pre
        .then(just("$"))
        .then(border1)
        .then(
            any()
                .and_is(border2.then(just("$")).not())
                .repeated()
                .collect::<String>(),
        )
        // .then(none_of("$").repeated().collect::<String>()) // todo: debug
        .then(border2)
        .then(just("$"))
        .then_ignore(post.rewind())
        .map_with(|(((((pre, d_pre), border1), body), border2), d_post), e| {
            e.state().prev_char = d_post.chars().last();
            let mut children = vec![];

            children.push(NodeOrToken::Token(GreenToken::new(
                OrgSyntaxKind::Dollar.into(),
                d_pre,
            )));

            let content = format!("{border1}{body}{border2}");

            children.push(NodeOrToken::Token(GreenToken::new(
                OrgSyntaxKind::Text.into(),
                &content,
            )));

            children.push(NodeOrToken::Token(GreenToken::new(
                OrgSyntaxKind::Dollar.into(),
                d_post,
            )));

            S2::Double(
                NodeOrToken::Token(GreenToken::new(
                    OrgSyntaxKind::Text.into(),
                    pre.to_string().as_str(),
                )),
                NodeOrToken::Node(GreenNode::new(
                    OrgSyntaxKind::LatexFragment.into(),
                    children,
                )),
            )
        });

    // \NAME [CONTENTS1]
    let t01 = just::<_, _, extra::Full<Rich<'_, char>, SimpleState<ParserState>, ()>>(r##"\"##)
        .then(name)
        .then(just("["))
        .then(
            none_of("{}[]\r\n")
                .and_is(just("]").not())
                .repeated()
                .collect::<String>(),
        )
        .then(just("]"))
        .map_with(|((((bs, name), lb), content), rb), e| {
            e.state().prev_char = rb.chars().last();
            let mut children = vec![];

            let _content = format!("{bs}{name}{lb}{content}{rb}");

            // children.push(NodeOrToken::Token(GreenToken::new(
            //     OrgSyntaxKind::BackSlash.into(),
            //     bs,
            // )));

            // children.push(NodeOrToken::Token(GreenToken::new(
            //     OrgSyntaxKind::Text.into(),
            //     &name,
            // )));

            // children.push(NodeOrToken::Token(GreenToken::new(
            //     OrgSyntaxKind::LeftSquareBracket.into(),
            //     lb,
            // )));

            children.push(NodeOrToken::Token(GreenToken::new(
                OrgSyntaxKind::Text.into(),
                &_content,
            )));

            // children.push(NodeOrToken::Token(GreenToken::new(
            //     OrgSyntaxKind::RightSquareBracket.into(),
            //     rb,
            // )));

            S2::Single(NodeOrToken::Node(GreenNode::new(
                OrgSyntaxKind::LatexFragment.into(),
                children,
            )))
        });

    // \NAME {CONTENTS2}
    let t02 = just(r##"\"##)
        .then(name)
        .then(just("{"))
        .then(
            none_of("{}\r\n")
                .and_is(just("}").not())
                .repeated()
                .collect::<String>(),
        )
        .then(just("}"))
        .map_with(|((((bs, name), lb), content), rb), e| {
            e.state().prev_char = rb.chars().last();
            let mut children = vec![];

            // children.push(NodeOrToken::Token(GreenToken::new(
            //     OrgSyntaxKind::BackSlash.into(),
            //     bs,
            // )));

            let _content = format!("{bs}{name}{lb}{content}{rb}");

            // children.push(NodeOrToken::Token(GreenToken::new(
            //     OrgSyntaxKind::Text.into(),
            //     &name,
            // )));

            // children.push(NodeOrToken::Token(GreenToken::new(
            //     OrgSyntaxKind::LeftCurlyBracket.into(),
            //     lb,
            // )));

            children.push(NodeOrToken::Token(GreenToken::new(
                OrgSyntaxKind::Text.into(),
                &_content,
            )));

            // children.push(NodeOrToken::Token(GreenToken::new(
            //     OrgSyntaxKind::RightCurlyBracket.into(),
            //     rb,
            // )));

            S2::Single(NodeOrToken::Node(GreenNode::new(
                OrgSyntaxKind::LatexFragment.into(),
                children,
            )))
        });

    choice((t1, t2, t3, t4, t5, t01, t02))
    // t1.or(t2).or(t3).or(t4).or(t5).or(t01).or(t02)
}
