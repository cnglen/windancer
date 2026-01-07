# object

|  | parser-rowan | parser-ast | html-render | comment |
|----|----|----|----|----|
| entity | ✓ | ✓ | ✓ |  |
| latex-fragment | ✓ | ✓ | ✓ |  |
| subscript | ✓ | ✓ | ✓ |  |
| superscript | ✓ | ✓ | ✓ |  |
| footnote-reference | ✓ | ✓ | ✓ |  |
| line-break | ✓ | ✓ | ✓ |  |
| macro | ✓ | ✓ |  | collect macro defintion from keyword |
| text-markup[^1] | ✓ | ✓ | ✓ |  |
| radio link | ✓ | ✓ | ✓ |  |
| angle link | ✓ | ✓ | ✓ |  |
| plain link | ✓ | ✓ | ✓ |  |
| regular link | ✓ | ✓ | ✓ |  |
| table-cell | ✓ | ✓ | ✓ |  |
| radio-target | ✓ | ✓ | ✓ |  |
| timestamp | ✓ | ✓ | ✓ |  |
| target | ✓ | ✓ | ✓ |  |
| inline-src-block | ✓ | ✓ |  |  |
| statistics-cookie |  |  |  | low |
| inline-babel-call |  |  |  | low |
| export-snippet |  |  |  | low |
| citation |  |  |  | low |
| citation-reference |  |  |  | low |

# element

|                     | parser-rowan | parser-ast | html-render | comment      |
|---------------------|--------------|------------|-------------|--------------|
| heading-subtree     | ✓            | ✓          | ✓           |              |
| section             | ✓            | ✓          | ✓           |              |
| drawer              | ✓            | ✓          | \-          |              |
| property-drawer     | ✓            | ✓          | \-          |              |
| center-block        | ✓            | ✓          | ✓           |              |
| quote-block         | ✓            | ✓          | ✓           |              |
| special-block       | ✓            | ✓          | ✓           |              |
| item                | ✓            | ✓          | ✓           |              |
| plain-List          | ✓            | ✓          | ✓           |              |
| Table               | ✓            | ✓          | ✓           |              |
| footnote-definition | ✓            | ✓          | ✓           |              |
| dynamic-block       |              |            |             | low          |
| inlinetask          |              |            |             | low          |
| comment             | ✓            | ✓          | ✓           |              |
| table-row           | ✓            | ✓          | ✓           |              |
| paragraph           | ✓            | ✓          | ✓           |              |
| node-property       | ✓            | ✓          | \-          |              |
| planning            | ✓            | ✓          | \-          |              |
| clock               |              |            |             | low          |
| diary-sexp          |              |            |             | low          |
| fixed-width         | ✓            | ✓          | ✓           |              |
| comment-block       | ✓            | ✓          | ✓           |              |
| example-block       | ✓            | ✓          | ✓           |              |
| verse-block         | ✓            | ✓          | ✓           |              |
| export-block        | ✓            | ✓          | ✓           |              |
| src-block           | ✓            | ✓          | ✓           |              |
| horizontal-rule     | ✓            | ✓          | ✓           |              |
| latex-environment   | ✓            | ✓          | ✓           |              |
| keyword             | ✓            | ✓          | \-          | babel~call~  |
| babel-call?         |              |            |             | low, \#+CALL |

# Footnotes

[^1]: bold, italic, underline, strike-through, code, verbatim
