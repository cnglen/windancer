//! Heading parser, including HeadingRow, HeadingSubtree
use crate::parser::ParserState;
use crate::parser::element::{drawer, planning, section};
use crate::parser::object;
use crate::parser::syntax::OrgSyntaxKind;
use chumsky::inspector::RollbackState;
use chumsky::prelude::*;
use rowan::{GreenNode, GreenToken, NodeOrToken};

pub(crate) fn heading_subtree_parser<'a>(
    element_parser: impl Parser<
        'a,
        &'a str,
        NodeOrToken<GreenNode, GreenToken>,
        extra::Full<Rich<'a, char>, RollbackState<ParserState>, usize>,
    > + Clone
    + 'a,
    prev_level: usize,
) -> impl Parser<
    'a,
    &'a str,
    NodeOrToken<GreenNode, GreenToken>,
    extra::Full<Rich<'a, char>, RollbackState<ParserState>, ()>,
> + Clone {
    let mut heading_subtree = Recursive::declare();

    let maybe_keyword_ws = choice((just("TODO"), just("DONE")))
        .then(object::whitespaces_g1())
        .or_not();
    let maybe_priority = just("[#")
        .then(one_of(
            "0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ",
        ))
        .then(just("]"))
        .then(object::whitespaces_g1())
        .or_not();
    let maybe_comment = just("COMMENT").then(object::whitespaces_g1()).or_not();
    let stars = just('*')
        .repeated()
        .configure(|cfg, ctx: &usize| cfg.at_least(ctx + 1))
        .to_slice()
        .map(|s: &str| s.len());
    let tags = just(":")
        .then(
            any()
                .filter(|c: &char| c.is_alphanumeric() || matches!(c, '_' | '#' | '@' | '%'))
                .repeated()
                .at_least(1)
                .to_slice()
                .separated_by(just(':'))
                .collect::<Vec<_>>(),
        )
        .then(just(":"))
        .then(object::whitespaces());
    let maybe_tag = tags.or_not();
    let maybe_title = none_of(object::CRLF)
        .and_is(
            one_of(" \t")
                .repeated()
                .at_least(1)
                .ignore_then(tags)
                .ignored()
                .not(),
        )
        .and_is(
            one_of(" \t")
                .repeated()
                .ignore_then(object::newline())
                .ignored()
                .not(),
        )
        .repeated()
        .at_least(1)
        .to_slice()
        .then(object::whitespaces())
        .or_not();
    let maybe_section_parser = section::section_parser(element_parser.clone()).or_not();
    heading_subtree.define(choice((stars
        .then_with_ctx(
            one_of(" \t")
                .repeated()
                .at_least(1)
                .to_slice()
                .then(maybe_keyword_ws)
                .then(maybe_priority)
                .then(maybe_comment)
                .then(maybe_title)
                .then(maybe_tag)
                .then(object::newline())
                .then(planning::planning_parser().or_not())
                .then(drawer::property_drawer_parser().or_not())
                .then(object::blank_line_parser().repeated().collect::<Vec<_>>())
                .then(maybe_section_parser.clone())
                .then(heading_subtree.clone().repeated().collect::<Vec<_>>()),
        )
        .map(
            |(
                stars,
                (
                    (
                        (
                            (
                                (
                                    (
                                        (
                                            (
                                                (
                                                    (
                                                        (whitespace1, maybe_keyword_ws),
                                                        maybe_priority,
                                                    ),
                                                    maybe_comment,
                                                ),
                                                maybe_title,
                                            ),
                                            maybe_tag,
                                        ),
                                        newline,
                                    ),
                                    maybe_planning,
                                ),
                                maybe_property_drawer,
                            ),
                            blanklines,
                        ),
                        maybe_section,
                    ),
                    subtrees,
                ),
            )| {
                let mut children = vec![];

                children.push(NodeOrToken::Token(GreenToken::new(
                    OrgSyntaxKind::HeadingRowStars.into(),
                    "*".repeat(stars).as_str(),
                    // "*".repeat(stars.prev_heading_level).as_str(),
                    // "*".repeat(e.ctx().prev_heading_level).as_str(),
                )));

                children.push(NodeOrToken::<GreenNode, GreenToken>::Token(
                    GreenToken::new(OrgSyntaxKind::Whitespace.into(), whitespace1),
                ));

                match maybe_keyword_ws {
                    Some((kw, ws)) if kw.to_uppercase() == "TODO" => {
                        children.push(NodeOrToken::<GreenNode, GreenToken>::Token(
                            GreenToken::new(OrgSyntaxKind::HeadingRowKeywordTodo.into(), kw),
                        ));
                        children.push(NodeOrToken::<GreenNode, GreenToken>::Token(
                            GreenToken::new(OrgSyntaxKind::Whitespace.into(), &ws),
                        ));
                    }
                    Some((kw, ws)) if kw.to_uppercase() == "DONE" => {
                        children.push(NodeOrToken::<GreenNode, GreenToken>::Token(
                            GreenToken::new(OrgSyntaxKind::HeadingRowKeywordDone.into(), kw),
                        ));

                        children.push(NodeOrToken::<GreenNode, GreenToken>::Token(
                            GreenToken::new(OrgSyntaxKind::Whitespace.into(), &ws),
                        ));
                    }

                    Some((kw, ws)) => {
                        children.push(NodeOrToken::<GreenNode, GreenToken>::Token(
                            GreenToken::new(OrgSyntaxKind::HeadingRowKeywordOther.into(), kw),
                        ));

                        children.push(NodeOrToken::<GreenNode, GreenToken>::Token(
                            GreenToken::new(OrgSyntaxKind::Whitespace.into(), &ws),
                        ));
                    }
                    None => {}
                }

                if let Some((((leftbracket_hash, level), rightbracket), whitespace)) =
                    maybe_priority
                {
                    let p_children = vec![
                        NodeOrToken::Token(GreenToken::new(
                            OrgSyntaxKind::LeftSquareBracket.into(),
                            &leftbracket_hash[0..1],
                        )),
                        NodeOrToken::Token(GreenToken::new(
                            OrgSyntaxKind::Hash.into(),
                            &leftbracket_hash[1..2],
                        )),
                        NodeOrToken::Token(GreenToken::new(
                            OrgSyntaxKind::Text.into(),
                            &level.to_string(),
                        )),
                        NodeOrToken::Token(GreenToken::new(
                            OrgSyntaxKind::RightSquareBracket.into(),
                            rightbracket,
                        )),
                    ];

                    let priority_node = NodeOrToken::<GreenNode, GreenToken>::Node(GreenNode::new(
                        OrgSyntaxKind::HeadingRowPriority.into(),
                        p_children,
                    ));

                    let ws_token = NodeOrToken::<GreenNode, GreenToken>::Token(GreenToken::new(
                        OrgSyntaxKind::Whitespace.into(),
                        whitespace,
                    ));

                    children.push(priority_node);
                    children.push(ws_token);
                }

                if let Some((comment, whitespace)) = maybe_comment {
                    children.push(NodeOrToken::<GreenNode, GreenToken>::Token(
                        GreenToken::new(OrgSyntaxKind::HeadingRowComment.into(), comment),
                    ));

                    children.push(NodeOrToken::<GreenNode, GreenToken>::Token(
                        GreenToken::new(OrgSyntaxKind::Whitespace.into(), whitespace),
                    ));
                }

                if let Some((title, whitespace)) = maybe_title {
                    let title_token = NodeOrToken::Token(GreenToken::new(
                        OrgSyntaxKind::HeadingRowTitle.into(),
                        title,
                    ));
                    children.push(title_token);

                    if !whitespace.is_empty() {
                        let ws_token = NodeOrToken::<GreenNode, GreenToken>::Token(
                            GreenToken::new(OrgSyntaxKind::Whitespace.into(), whitespace),
                        );
                        children.push(ws_token);
                    }
                }

                if let Some((((left_colon, tags), right_colon), whitespace)) = maybe_tag {
                    let mut tag_token_children: Vec<NodeOrToken<GreenNode, GreenToken>> = vec![];
                    tag_token_children.push(NodeOrToken::Token(GreenToken::new(
                        OrgSyntaxKind::Colon.into(),
                        &left_colon.to_string(),
                    )));

                    for tag in tags {
                        tag_token_children.push(NodeOrToken::Token(GreenToken::new(
                            OrgSyntaxKind::HeadingRowTag.into(),
                            tag,
                        )));

                        tag_token_children.push(NodeOrToken::Token(GreenToken::new(
                            OrgSyntaxKind::Colon.into(),
                            right_colon,
                        )));
                    }

                    let tag_node: NodeOrToken<GreenNode, GreenToken> = NodeOrToken::Node(
                        GreenNode::new(OrgSyntaxKind::HeadingRowTags.into(), tag_token_children),
                    );
                    children.push(tag_node);

                    if whitespace.len() > 0 {
                        children.push(NodeOrToken::<GreenNode, GreenToken>::Token(
                            GreenToken::new(OrgSyntaxKind::Whitespace.into(), whitespace),
                        ));
                    }
                }

                children.push(NodeOrToken::Token(GreenToken::new(
                    OrgSyntaxKind::Newline.into(),
                    newline,
                )));

                if let Some(planning) = maybe_planning {
                    children.push(planning);
                }

                if let Some(property_drawer) = maybe_property_drawer {
                    children.push(property_drawer);
                }

                children.extend(blanklines);
                let head_row = NodeOrToken::<GreenNode, GreenToken>::Node(GreenNode::new(
                    OrgSyntaxKind::HeadingRow.into(),
                    children,
                ));

                let mut children = vec![];
                children.push(head_row);
                if let Some(section) = maybe_section {
                    children.push(section);
                }
                for subtree in subtrees {
                    children.push(subtree);
                }
                NodeOrToken::Node(GreenNode::new(
                    OrgSyntaxKind::HeadingSubtree.into(),
                    children,
                ))
            },
        ),)));
    heading_subtree.with_ctx(prev_level).boxed()
}

/// A simple heading row parser WITHOUT state, ONLY used for look ahead
// - section parser: to check whether the next part is heading to stop
pub(crate) fn simple_heading_row_parser<'a, C: 'a>()
-> impl Parser<'a, &'a str, &'a str, extra::Full<Rich<'a, char>, RollbackState<ParserState>, C>> + Clone
{
    let stars = just('*').repeated().at_least(1);
    let whitespaces = one_of(" \t").repeated().at_least(1);
    let title = none_of(object::CRLF).repeated();
    stars
        .ignore_then(whitespaces)
        .ignore_then(title)
        .ignore_then(object::newline_or_ending())
        .to_slice()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::common::{get_parser_output, get_parsers_output};
    use crate::parser::element::element_parser;
    use pretty_assertions::assert_eq;

    #[test]
    fn test_heading_subtree_01() {
        let input = "* 标题1\n 测试\n** 标题1.1\n测试\n测试\ntest \n*** 1.1.1 title\nContent\n";
        let parser = heading_subtree_parser(element_parser(), 0);
        assert_eq!(
            get_parser_output(parser, input),
            r##"HeadingSubtree@0..75
  HeadingRow@0..10
    HeadingRowStars@0..1 "*"
    Whitespace@1..2 " "
    HeadingRowTitle@2..9 "标题1"
    Newline@9..10 "\n"
  Section@10..18
    Paragraph@10..18
      Text@10..18 " 测试\n"
  HeadingSubtree@18..75
    HeadingRow@18..31
      HeadingRowStars@18..20 "**"
      Whitespace@20..21 " "
      HeadingRowTitle@21..30 "标题1.1"
      Newline@30..31 "\n"
    Section@31..51
      Paragraph@31..51
        Text@31..51 "测试\n测试\ntest \n"
    HeadingSubtree@51..75
      HeadingRow@51..67
        HeadingRowStars@51..54 "***"
        Whitespace@54..55 " "
        HeadingRowTitle@55..66 "1.1.1 title"
        Newline@66..67 "\n"
      Section@67..75
        Paragraph@67..75
          Text@67..75 "Content\n"
"##
        );
    }

    #[test]
    fn test_heading_subtree_02() {
        let input = "* 标题1\n 测试\n** 标题1.1\n测试\n测试\ntest\n*** 1.1.1 title\nContent\n* Title\nI have a dream\n"; // overflow
        let parser = heading_subtree_parser(element_parser(), 0)
            .repeated()
            .collect::<Vec<_>>();
        assert_eq!(
            get_parsers_output(parser, input),
            r##"Root@0..97
  HeadingSubtree@0..74
    HeadingRow@0..10
      HeadingRowStars@0..1 "*"
      Whitespace@1..2 " "
      HeadingRowTitle@2..9 "标题1"
      Newline@9..10 "\n"
    Section@10..18
      Paragraph@10..18
        Text@10..18 " 测试\n"
    HeadingSubtree@18..74
      HeadingRow@18..31
        HeadingRowStars@18..20 "**"
        Whitespace@20..21 " "
        HeadingRowTitle@21..30 "标题1.1"
        Newline@30..31 "\n"
      Section@31..50
        Paragraph@31..50
          Text@31..50 "测试\n测试\ntest\n"
      HeadingSubtree@50..74
        HeadingRow@50..66
          HeadingRowStars@50..53 "***"
          Whitespace@53..54 " "
          HeadingRowTitle@54..65 "1.1.1 title"
          Newline@65..66 "\n"
        Section@66..74
          Paragraph@66..74
            Text@66..74 "Content\n"
  HeadingSubtree@74..97
    HeadingRow@74..82
      HeadingRowStars@74..75 "*"
      Whitespace@75..76 " "
      HeadingRowTitle@76..81 "Title"
      Newline@81..82 "\n"
    Section@82..97
      Paragraph@82..97
        Text@82..97 "I have a dream\n"
"##
        );
    }
}
