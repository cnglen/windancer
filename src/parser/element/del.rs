    // let heading_subtree_without_subtree = stars
    //     .then(one_of(" \t").repeated().at_least(1))
    //     .then(maybe_keyword_ws)
    //     .then(maybe_priority)
    //     .then(maybe_comment)
    //     .then(maybe_title)
    //     .then(maybe_tag)
    //     .then(object::newline_v2())
    //     .then(planning::planning_parser().or_not())
    //     .then(drawer::property_drawer_parser().or_not())
    //     .then(object::blank_line_parser().repeated().collect::<Vec<_>>())
    //     .map(
    //         |(
    //             (
    //                 (
    //                     (
    //                         (
    //                             (
    //                                 (
    //                                     (((stars, whitespace1), maybe_keyword_ws), maybe_priority),
    //                                     maybe_comment,
    //                                 ),
    //                                 maybe_title,
    //                             ),
    //                             maybe_tag,
    //                         ),
    //                         newline,
    //                     ),
    //                     maybe_planning,
    //                 ),
    //                 maybe_property_drawer,
    //             ),
    //             blanklines,
    //         )| {
    //             let mut children = vec![];

    //             children.push(NodeOrToken::Token(GreenToken::new(
    //                 OrgSyntaxKind::HeadingRowStars.into(),
    //                 &stars,
    //             )));

    //             match maybe_keyword_ws {
    //                 Some((kw, ws)) if kw.to_uppercase() == "TODO" => {
    //                     children.push(NodeOrToken::<GreenNode, GreenToken>::Token(
    //                         GreenToken::new(OrgSyntaxKind::HeadingRowKeywordTodo.into(), kw),
    //                     ));
    //                     children.push(NodeOrToken::<GreenNode, GreenToken>::Token(
    //                         GreenToken::new(OrgSyntaxKind::Whitespace.into(), &ws),
    //                     ));
    //                 }
    //                 Some((kw, ws)) if kw.to_uppercase() == "DONE" => {
    //                     children.push(NodeOrToken::<GreenNode, GreenToken>::Token(
    //                         GreenToken::new(OrgSyntaxKind::HeadingRowKeywordDone.into(), kw),
    //                     ));

    //                     children.push(NodeOrToken::<GreenNode, GreenToken>::Token(
    //                         GreenToken::new(OrgSyntaxKind::Whitespace.into(), &ws),
    //                     ));
    //                 }

    //                 Some((kw, ws)) => {
    //                     children.push(NodeOrToken::<GreenNode, GreenToken>::Token(
    //                         GreenToken::new(OrgSyntaxKind::HeadingRowKeywordOther.into(), kw),
    //                     ));

    //                     children.push(NodeOrToken::<GreenNode, GreenToken>::Token(
    //                         GreenToken::new(OrgSyntaxKind::Whitespace.into(), &ws),
    //                     ));
    //                 }
    //                 None => {}
    //             }

    //             children.push(NodeOrToken::Token(GreenToken::new(
    //                 OrgSyntaxKind::Newline.into(),
    //                 newline,
    //             )));

    //             if let Some(planning) = maybe_planning {
    //                 children.push(planning);
    //             }

    //             if let Some(property_drawer) = maybe_property_drawer {
    //                 children.push(property_drawer);
    //             }

    //             for blankline in blanklines {
    //                 children.push(NodeOrToken::Token(blankline))
    //             }

    //             NodeOrToken::<GreenNode, GreenToken>::Node(GreenNode::new(OrgSyntaxKind::HeadingRow.into(), children))
    //         },
    //     )
    //     .then(
    //         // element_parser: C VS (String, &str, &str)
    //         // section::section_parser::<ParserContext>(element_parser.clone()).or_not(),
    //         object::line_parser()
    //             .and_is(just("*").repeated().at_least(1).then(just(" ")).not())
    //             .repeated()
    //             .to_slice()
    //             .map(|s| {
    //                 let token = NodeOrToken::<GreenNode, GreenToken>::Token(GreenToken::new(
    //                     OrgSyntaxKind::Text.into(),
    //                     s,
    //                 ));

    //                 NodeOrToken::<GreenNode, GreenToken>::Node(GreenNode::new(OrgSyntaxKind::Section.into(), vec![token]))
    //             }).or_not(), // just("asfd").to_slice(),
    //     )
    //     .map(|(row, maybe_section)| {

    //         let mut children = vec![];
    //         // children.push(row.clone());
    //         // if let Some(section) = maybe_section {
    //         //     children.push(section)
    //         // }
    //         let row: SyntaxNode<OrgLanguage> = SyntaxNode::new_root(row.as_node().expect("xx").clone());
    //         let prev_heading_level = row.first_child_or_token_by_kind(&|e| e == OrgSyntaxKind::HeadingRowStars)
    //             .expect("stars")
    //             .into_token()
    //             .expect("stars into token")
    //             .text()
    //             .len();
                
    //         ParserContext {
    //             prev_heading_level,
    //             // node: NodeOrToken::Node(GreenNode::new(
    //             //     OrgSyntaxKind::HeadingSubtree.into(),
    //             //     children,
    //             // ))
    //         }
    //     });


        // heading_subtree_without_subtree.clone().then_ignore(end())
        //     .map(|ctx| {
        //         ctx.node
        //     }),
            
        // heading_subtree_without_subtree
        //     .then_with_ctx(heading_subtree.clone().repeated().collect::<Vec<_>>())
        //     .map(|(ctx, subtrees)| {
        //         let mut children = vec![];
        //         // for child in ctx.node.into_node().expect("xx").children() {
        //         //     children.push(child.to_owned());
        //         // }
        //         // for e in subtrees {
        //         //     children.push(e);
        //         // }

        //         NodeOrToken::Node(GreenNode::new(
        //             OrgSyntaxKind::HeadingSubtree.into(),
        //             children,
        //         ))
        //     }),
