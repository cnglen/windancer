An [org-mode](https://orgmode.org/) toolkit, including parser, renderer.

# Status

STILL IN DEVELOPMENT.

- ☒ basic org-mode parser: string -parse-\> AST
- ☒ html renderer: AST to html
- \[\] site generator

## <span class="todo TODO">TODO</span> syntax check list [syntax-check-list]

|  | kind | parser | ast | html-render | comment |
|----|----|----|----|----|----|
| Entity | object | ✓ | ✓ | ✓ |  |
| LatexFragment | object | ✓ | ✓ | ✓ |  |
| FootnoteReference | object | ✓ | ✓ | ✓ | Parse of **definition** is simplified[^1] |
| TextMarkup | object |  |  |  |  |
| Link | object |  |  |  |  |
| InlineSourceBlock | object |  |  |  |  |
| TableCell | object |  |  |  |  |
| HeadingSubtree | greater element |  |  |  |  |
| Section | greater element |  |  |  |  |
| Table |  |  |  |  |  |
| Drawer |  |  |  |  |  |
| CenterBlock |  |  |  |  |  |
| QuoteBlock |  |  |  |  |  |
| SpecialBlock |  |  |  |  |  |
| List |  |  |  |  |  |
| Item |  |  |  |  |  |
| FootnoteDefinition |  |  |  |  |  |
| Paragraph |  |  |  |  |  |
| SrcBlock |  |  |  |  |  |
| CommentBlock |  |  |  |  |  |
| VerseBlock |  |  |  |  |  |
| ExampleBlock |  |  |  |  |  |
| HorizontalRule |  |  |  |  |  |
| LatexEnvironment |  |  |  |  |  |
| Keyrord |  |  |  |  |  |
| TableRow |  |  |  |  |  |

## test

### <span class="todo TODO">TODO</span> API design [api-design]

``` bash
cargo run
cargo test
```

# Usage

``` bash
cargo install windancer
windancer --help
```

## examples

input:

- org~file~

output:

- html

# Design

org -parser-\> syntax tree(red tree) -ast~builder~-\> ast
-html~renderer~-\> html –orgbook/mdbook –\> site

- rowan red/green tree
- chumsky
- render
- html

windancer -i xxx.org -o xxx.html

``` rust
let green_tree = parser.parse(f_org).green();
let red_tree = parser.parse(f_org).syntax();
let ast = ast_builder.build(red_tree);
let html = html_render.render(ast);
```

## module

parser
:   parse org-mode doc into GreenTree, the generate RedTree(syntax tree)
using rowan's API

ast
:   build AST from SyntaxTree

render
:   render html from AST

# Reference

- [org-syntax](https://orgmode.org/worg/org-syntax.html)
- [chumsky](https://github.com/zesterer/chumsky)
- [orgize](https://github.com/tfeldmann/organize)
- mdbook

# Footnotes

[^1]: In inline and anonymous footnote, DEFINITION is One or more
**objects** from the standard set, simplified to use text, i.e,
​`any().and_is(just("]").not()).repeated().collect::<String>();`​
