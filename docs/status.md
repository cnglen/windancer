# object

|  | parser-rowan | parser-ast | html-render | comment |
|----|----|----|----|----|
| entity | ✓ | ✓ | ✓ |  |
| latex-fragment | ✓ | ✓ | ✓ |  |
| footnote-reference | ✓ | ✓ | ✓ | Parse of **definition** is simplified[^1] |
| line-break | ✓ | ✓ | ✓ |  |
| macro | ✓ | ✓ |  | todo render: collect macro replacement |
| text-markup[^2] |  |  |  | todo |
| radio link |  |  |  |  |
| angle link | ✓ |  |  |  |
| plain link |  |  |  |  |
| regular link |  |  |  |  |
| table-cell |  |  |  | todo |
| subscript |  |  |  | todo |
| superscript | ✓ |  |  | parse of expression is simplified[^3] |
| citation |  |  |  | todo |
| citation-reference |  |  |  | todo |
| radio-target |  |  |  | todo |
| timestamp | ✓ |  |  | todo |
| target | ✓ |  |  | todo |
| statistics-cookie |  |  |  | low |
| inline-babel-call |  |  |  | low |
| export-snippet |  |  |  | low |
| inline-src-block |  |  |  | low-priority |

# element

## greater element

|                    | parser-rowan | parser-ast | html-render | comment |
|--------------------|--------------|------------|-------------|---------|
| HeadingSubtree     |              |            |             |         |
| Section            |              |            |             |         |
| Table              |              |            |             |         |
| Drawer             |              |            |             |         |
| CenterBlock        |              |            |             |         |
| QuoteBlock         |              |            |             |         |
| SpecialBlock       |              |            |             |         |
| List               |              |            |             |         |
| Item               |              |            |             |         |
| FootnoteDefinition |              |            |             |         |
| Paragraph          |              |            |             |         |
| SrcBlock           |              |            |             |         |
| CommentBlock       |              |            |             |         |
| VerseBlock         |              |            |             |         |
| ExampleBlock       |              |            |             |         |
| HorizontalRule     |              |            |             |         |
| LatexEnvironment   |              |            |             |         |
| Keyrord            |              |            |             |         |
| TableRow           |              |            |             |         |

## lesser element

# Footnotes

[^1]: In inline and anonymous footnote, DEFINITION is One or more
**objects** from the standard set, simplified to use text, i.e,
​`any().and_is(just("]").not()).repeated().collect::<String>();`​

[^2]: bold, italic, underline, strike-throught, code, verbatim

[^3]: An expression enclosed in curly brackets ({, }) or in round braces
((, )), which may itself contain balanced curly or round brackets
and the standard set of objects. Simplified to use text, to be
updated.
