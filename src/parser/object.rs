//! Object paser (todo)

use crate::parser::ParserResult;
use crate::parser::ParserState;
use crate::parser::S2;
use crate::parser::markup::text_markup_parser;
use crate::parser::syntax::OrgSyntaxKind;

use chumsky::input::InputRef;
use chumsky::inspector::SimpleState;
use chumsky::prelude::*;
use rowan::{GreenNode, GreenToken, NodeOrToken};
use std::ops::Range;
// use chumsky::input::InputRef;

use phf::phf_map;
use std::collections::HashMap;

pub(crate) static entityname_to_html: phf::Map<&'static str, &'static str> = phf_map! {
        // * letters
        // ** latin
        "Agrave" => "&Agrave;",
        "agrave" => "&agrave;",
        "Aacute" => "&Aacute;",
        "aacute" => "&aacute;",
        "Acirc" => "&Acirc;",
        "acirc" => "&acirc;",
        "Amacr" => "&Amacr;",
        "amacr" => "&amacr;",
        "Atilde" => "&Atilde;",
        "atilde" => "&atilde;",
        "Auml" => "&Auml;",
        "auml" => "&auml;",
        "Aring" => "&Aring;",
        "AA" => "&Aring;",
        "aring" => "&aring;",
        "AElig" => "&AElig;",
        "aelig" => "&aelig;",
        "Ccedil" => "&Ccedil;",
        "ccedil" => "&ccedil;",
        "Egrave" => "&Egrave;",
        "egrave" => "&egrave;",
        "Eacute" => "&Eacute;",
        "eacute" => "&eacute;",
        "Ecirc" => "&Ecirc;",
        "ecirc" => "&ecirc;",
        "Euml" => "&Euml;",
        "euml" => "&euml;",
        "Igrave" => "&Igrave;",
        "igrave" => "&igrave;",
        "Iacute" => "&Iacute;",
        "iacute" => "&iacute;",
        "Idot" => "&idot;",
        "inodot" => "&inodot;",
        "Icirc" => "&Icirc;",
        "icirc" => "&icirc;",
        "Iuml" => "&Iuml;",
        "iuml" => "&iuml;",
        "Ntilde" => "&Ntilde;",
        "ntilde" => "&ntilde;",
        "Ograve" => "&Ograve;",
        "ograve" => "&ograve;",
        "Oacute" => "&Oacute;",
        "oacute" => "&oacute;",
        "Ocirc" => "&Ocirc;",
        "ocirc" => "&ocirc;",
        "Otilde" => "&Otilde;",
        "otilde" => "&otilde;",
        "Ouml" => "&Ouml;",
        "ouml" => "&ouml;",
        "Oslash" => "&Oslash;",
        "oslash" => "&oslash;",
        "OElig" => "&OElig;",
        "oelig" => "&oelig;",
        "Scaron" => "&Scaron;",
        "scaron" => "&scaron;",
        "szlig" => "&szlig;",
        "Ugrave" => "&Ugrave;",
        "ugrave" => "&ugrave;",
        "Uacute" => "&Uacute;",
        "uacute" => "&uacute;",
        "Ucirc" => "&Ucirc;",
        "ucirc" => "&ucirc;",
        "Uuml" => "&Uuml;",
        "uuml" => "&uuml;",
        "Yacute" => "&Yacute;",
        "yacute" => "&yacute;",
        "Yuml" => "&Yuml;",
        "yuml" => "&yuml;",

        // ** Latin special face,"
        "fnof" => "&fnof;",
        "real" => "&real;",
        "image" => "&image;",
        "weierp" => "&weierp;",
        "ell" => "&ell;",
        "imath" => "&imath;",
        "jmath" => "&jmath;",

        // ** Greek"
        "Alpha" => "&Alpha;",
        "alpha" => "&alpha;",
        "Beta" => "&Beta;",
        "beta" => "&beta;",
        "Gamma" => "&Gamma;",
        "gamma" => "&gamma;",
        "Delta" => "&Delta;",
        "delta" => "&delta;",
        "Epsilon" => "&Epsilon;",
        "epsilon" => "&epsilon;",
        "varepsilon" => "&epsilon;",
        "Zeta" => "&Zeta;",
        "zeta" => "&zeta;",
        "Eta" => "&Eta;",
        "eta" => "&eta;",
        "Theta" => "&Theta;",
        "theta" => "&theta;",
        "thetasym" => "&thetasym;",
        "vartheta" => "&thetasym;",
        "Iota" => "&Iota;",
        "iota" => "&iota;",
        "Kappa" => "&Kappa;",
        "kappa" => "&kappa;",
        "Lambda" => "&Lambda;",
        "lambda" => "&lambda;",
        "Mu" => "&Mu;",
        "mu" => "&mu;",
        "nu" => "&nu;",
        "Nu" => "&Nu;",
        "Xi" => "&Xi;",
        "xi" => "&xi;",
        "Omicron" => "&Omicron;",
        "omicron" => "&omicron;",
        "Pi" => "&Pi;",
        "pi" => "&pi;",
        "Rho" => "&Rho;",
        "rho" => "&rho;",
        "Sigma" => "&Sigma;",
        "sigma" => "&sigma;",
        "sigmaf" => "&sigmaf;",
        "varsigma" => "&sigmaf;",
        "Tau" => "&Tau;",
        "Upsilon" => "&Upsilon;",
        "upsih" => "&upsih;",
        "upsilon" => "&upsilon;",
        "Phi" => "&Phi;",
        "phi" => "&phi;",
        "varphi" => "&varphi;",
        "Chi" => "&Chi;",
        "chi" => "&chi;",
        "acutex" => "&acute;x",
        "Psi" => "&Psi;",
        "psi" => "&psi;",
        "tau" => "&tau;",
        "Omega" => "&Omega;",
        "omega" => "&omega;",
        "piv" => "&piv;",
        "varpi" => "&piv;",
        "partial" => "&part;",

        // ** Hebrew"
        "alefsym" => "&alefsym;",
        "aleph" => "&aleph;",
        "gimel" => "&gimel;",
        "beth" => "&beth;",
        "dalet" => "&daleth;",

        // ** Icelandic"
        "ETH" => "&ETH;",
        "eth" => "&eth;",
        "THORN" => "&THORN;",
        "thorn" => "&thorn;",

        // * Punctuation"
        // ** Dots and Marks"
        "dots" => "&hellip;",
        "cdots" => "&ctdot;",
        "hellip" => "&hellip;",
        "middot" => "&middot;",
        "iexcl" => "&iexcl;",
        "iquest" => "&iquest;",

        // ** Dash-like"
        "shy" => "&shy;",
        "ndash" => "&ndash;",
        "mdash" => "&mdash;",

        // ** Quotations"
        "quot" => "&quot;",
        "acute" => "&acute;",
        "ldquo" => "&ldquo;",
        "rdquo" => "&rdquo;",
        "bdquo" => "&bdquo;",
        "lsquo" => "&lsquo;",
        "rsquo" => "&rsquo;",
        "sbquo" => "&sbquo;",
        "laquo" => "&laquo;",
        "raquo" => "&raquo;",
        "lsaquo" => "&lsaquo;",
        "rsaquo" => "&rsaquo;",

        // * Other"
        // ** Misc. often used"
        "circ" => "&circ;",
        "vert" => "&vert;",
        "vbar" => "|",
        "brvbar" => "&brvbar;",
        "S" => "&sect;",
        "sect" => "&sect;",
        "P" => "&para;",
        "para" => "&para;",
        "amp" => "&amp;",
        "lt" => "&lt;",
        "gt" => "&gt;",
        "tilde" => "~",
        "slash" => "/",
        "plus" => "+",
        "under" => "_",
        "equal" => "=",
        "asciicirc" => "^",
        "dagger" => "&dagger;",
        "dag" => "&dagger;",
        "Dagger" => "&Dagger;",
        "ddag" => "&Dagger;",

        // ** Whitespace"
        "nbsp" => "&nbsp;",
        "ensp" => "&ensp;",
        "emsp" => "&emsp;",
        "thinsp" => "&thinsp;",

        // ** Currency"
        "curren" => "&curren;",
        "cent" => "&cent;",
        "pound" => "&pound;",
        "yen" => "&yen;",
        "euro" => "&euro;",
        "EUR" => "&euro;",
        "dollar" => "$",
        "USD" => "$",

        // ** Property Marks"
        "copy" => "&copy;",
        "reg" => "&reg;",
        "trade" => "&trade;",

        // ** Science et al."
        "minus" => "&minus;",
        "pm" => "&plusmn;",
        "plusmn" => "&plusmn;",
        "times" => "&times;",
        "frasl" => "&frasl;",
        "colon" => ":",
        "div" => "&divide;",
        "frac12" => "&frac12;",
        "frac14" => "&frac14;",
        "frac34" => "&frac34;",
        "permil" => "&permil;",
        "sup1" => "&sup1;",
        "sup2" => "&sup2;",
        "sup3" => "&sup3;",
        "radic" => "&radic;",
        "sum" => "&sum;",
        "prod" => "&prod;",
        "micro" => "&micro;",
        "macr" => "&macr;",
        "deg" => "&deg;",
        "prime" => "&prime;",
        "Prime" => "&Prime;",
        "infin" => "&infin;",
        "infty" => "&infin;",
        "prop" => "&prop;",
        "propto" => "&prop;",
        "not" => "&not;",
        "neg" => "&not;",
        "land" => "&and;",
        "wedge" => "&and;",
        "lor" => "&or;",
        "vee" => "&or;",
        "cap" => "&cap;",
        "cup" => "&cup;",
        "smile" => "&smile;",
        "frown" => "&frown;",
        "int" => "&int;",
        "therefore" => "&there4;",
        "there4" => "&there4;",
        "because" => "&because;",
        "sim" => "&sim;",
        "cong" => "&cong;",
        "simeq" => "&cong;" ,
        "asymp" => "&asymp;",
        "approx" => "&asymp;",
        "ne" => "&ne;",
        "neq" => "&ne;",
        "equiv" => "&equiv;",

        "triangleq" => "&triangleq;",
        "le" => "&le;",
        "leq" => "&le;",
        "ge" => "&ge;",
        "geq" => "&ge;",
        "lessgtr" => "&lessgtr;",
        "lesseqgtr" => "&lesseqgtr;",
        "ll" => "&Lt;",
        "Ll" => "&Ll;",
        "lll" => "&Ll;",
        "gg" => "&Gt;",
        "Gg" => "&Gg;",
        "ggg" => "&Gg;",
        "prec" => "&pr;",
        "preceq" => "&prcue;",
        "preccurlyeq" => "&prcue;",
        "succ" => "&sc;",
        "succeq" => "&sccue;",
        "succcurlyeq" => "&sccue;",
        "sub" => "&sub;",
        "subset" => "&sub;",
        "sup" => "&sup;",
        "supset" => "&sup;",
        "nsub" => "&nsub;",
        "sube" => "&sube;",
        "nsup" => "&nsup;",
        "supe" => "&supe;",
        "setminus" => "&setminus;",
        "forall" => "&forall;",
        "exist" => "&exist;",
        "exists" => "&exist;",
        "nexist" => "&exist;",
        "nexists" => "&exist;",
        "empty" => "&empty;",
        "emptyset" => "&empty;",
        "isin" => "&isin;",
        "in" => "&isin;",
        "notin" => "&notin;",
        "ni" => "&ni;",
        "nabla" => "&nabla;",
        "ang" => "&ang;",
        "angle" => "&ang;",
        "perp" => "&perp;",
        "parallel" => "&parallel;",
        "sdot" => "&sdot;",
        "cdot" => "&sdot;",
        "lceil" => "&lceil;",
        "rceil" => "&rceil;",
        "lfloor" => "&lfloor;",
        "rfloor" => "&rfloor;",
        "lang" => "&lang;",
        "rang" => "&rang;",
        "langle" => "&lang;",
        "rangle" => "&rang;",
        "hbar" => "&hbar;",
        "mho" => "&mho;",

        // ** Arrows"
        "larr" => "&larr;",
        "leftarrow" => "&larr;" ,
        "gets" => "&larr;" ,
        "lArr" => "&lArr;",
        "Leftarrow" => "&lArr;",
        "uarr" => "&uarr;",
        "uparrow" => "&uarr;",
        "uArr" => "&uArr;",
        "Uparrow" => "&uArr;",
        "rarr" => "&rarr;",
        "to" => "&rarr;",
        "rightarrow" => "&rarr;" ,
        "rArr" => "&rArr;",
        "Rightarrow" => "&rArr;",
        "darr" => "&darr;",
        "downarrow" => "&darr;",
        "dArr" => "&dArr;",
        "Downarrow" => "&dArr;",
        "harr" => "&harr;",
        "leftrightarrow" => "&harr;" ,
        "hArr" => "&hArr;",
        "Leftrightarrow" => "&hArr;",
        "crarr" => "&crarr;",
        "hookleftarrow" => "&crarr;" ,

        // ** Function names"
        "arccos" => "arccos",
        "arcsin" => "arcsin",
        "arctan" => "arctan",
        "arg" => "arg",
        "cos" => "cos",
        "cosh" => "cosh",
        "cot" => "cot",
        "coth" => "coth",
        "csc" => "csc",
        "det" => "det",
        "dim" => "dim",
        "exp" => "exp",
        "gcd" => "gcd",
        "hom" => "hom",
        "inf" => "inf",
        "ker" => "ker",
        "lg" => "lg",
        "lim" => "lim",
        "liminf" => "liminf",
        "limsup" => "limsup",
        "ln" => "ln",
        "log" => "log",
        "max" => "max",
        "min" => "min",
        "Pr" => "Pr",
        "sec" => "sec",
        "sin" => "sin",
        "sinh" => "sinh",
        "tan" => "tan",
        "tanh" => "tanh",

        // ** Signs & Symbols"
        "bull" => "&bull;",
        "bullet" => "&bull;",
        "star" => "*",
        "lowast" => "&lowast;",
        "ast" => "&lowast;",
        "odot" => "o",
        "oplus" => "&oplus;",
        "otimes" => "&otimes;",
        "check" => "&checkmark;",
        "checkmark" => "&check;",

        // ** Miscellaneous seldom used,"
        "ordf" => "&ordf;",
        "ordm" => "&ordm;",
        "cedil" => "&cedil;",
        "oline" => "&oline;",
        "uml" => "&uml;",
        "zwnj" => "&zwnj;",
        "zwj" => "&zwj;",
        "lrm" => "&lrm;",
        "rlm" => "&rlm;",

        // ** Smilies"
        "smiley" => "&#9786;",
        "blacksmile" => "&#9787;",
        "sad" => "&#9785;",
        "frowny" => "&#9785;",

        // ** Suits"
        "clubs" => "&clubs;",
        "clubsuit" => "&clubs;",
        "spades" => "&spades;",
        "spadesuit" => "&spades;",
        "hearts" => "&hearts;",
        "heartsuit" => "&heartsuit;",
        "diams" => "&diams;",
        "diamondsuit" => "&diams;",
        "diamond" => "&diamond;",
        "Diamond" => "&diamond;",
        "loz" => "&loz;",

    // spaces
    "_ " => "&ensp;",
    "_  " => "&ensp;&ensp;",
    "_   " => "&ensp;&ensp;&ensp;",
    "_    " => "&ensp;&ensp;&ensp;&ensp;",
    "_     " => "&ensp;&ensp;&ensp;&ensp;&ensp;",
    "_      " => "&ensp;&ensp;&ensp;&ensp;&ensp;&ensp;",
    "_       " => "&ensp;&ensp;&ensp;&ensp;&ensp;&ensp;&ensp;",
    "_        " => "&ensp;&ensp;&ensp;&ensp;&ensp;&ensp;&ensp;&ensp;",
    "_         " => "&ensp;&ensp;&ensp;&ensp;&ensp;&ensp;&ensp;&ensp;&ensp;",
    "_          " => "&ensp;&ensp;&ensp;&ensp;&ensp;&ensp;&ensp;&ensp;&ensp;&ensp;",
    "_           " => "&ensp;&ensp;&ensp;&ensp;&ensp;&ensp;&ensp;&ensp;&ensp;&ensp;&ensp;",
    "_            " => "&ensp;&ensp;&ensp;&ensp;&ensp;&ensp;&ensp;&ensp;&ensp;&ensp;&ensp;&ensp;",
    "_             " => "&ensp;&ensp;&ensp;&ensp;&ensp;&ensp;&ensp;&ensp;&ensp;&ensp;&ensp;&ensp;&ensp;",
    "_              " => "&ensp;&ensp;&ensp;&ensp;&ensp;&ensp;&ensp;&ensp;&ensp;&ensp;&ensp;&ensp;&ensp;&ensp;",
    "_               " => "&ensp;&ensp;&ensp;&ensp;&ensp;&ensp;&ensp;&ensp;&ensp;&ensp;&ensp;&ensp;&ensp;&ensp;&ensp;",
    "_                " => "&ensp;&ensp;&ensp;&ensp;&ensp;&ensp;&ensp;&ensp;&ensp;&ensp;&ensp;&ensp;&ensp;&ensp;&ensp;&ensp;",
    "_                 " => "&ensp;&ensp;&ensp;&ensp;&ensp;&ensp;&ensp;&ensp;&ensp;&ensp;&ensp;&ensp;&ensp;&ensp;&ensp;&ensp;&ensp;",
    "_                  " => "&ensp;&ensp;&ensp;&ensp;&ensp;&ensp;&ensp;&ensp;&ensp;&ensp;&ensp;&ensp;&ensp;&ensp;&ensp;&ensp;&ensp;&ensp;",
    "_                   " => "&ensp;&ensp;&ensp;&ensp;&ensp;&ensp;&ensp;&ensp;&ensp;&ensp;&ensp;&ensp;&ensp;&ensp;&ensp;&ensp;&ensp;&ensp;&ensp;",
    "_                    " => "&ensp;&ensp;&ensp;&ensp;&ensp;&ensp;&ensp;&ensp;&ensp;&ensp;&ensp;&ensp;&ensp;&ensp;&ensp;&ensp;&ensp;&ensp;&ensp;&ensp;",


};

/// 解析行终止符：换行符或输入结束
pub(crate) fn newline_or_ending<'a>()
-> impl Parser<'a, &'a str, Option<String>, extra::Full<Rich<'a, char>, SimpleState<ParserState>, ()>>
+ Clone {
    just('\n').map(|c| Some(String::from(c))).or(end().to(None))
}

/// 创建一个不区分大小写的关键字解析器
pub(crate) fn just_case_insensitive<'a>(
    s: &'a str,
) -> impl Parser<'a, &'a str, String, extra::Full<Rich<'a, char>, SimpleState<ParserState>, ()>> + Clone
{
    let s_lower = s.to_lowercase();

    any()
        .filter(|c: &char| c.is_ascii())
        .repeated()
        .exactly(s.chars().count())
        .collect::<String>()
        .try_map_with(move |t, e| {
            if t.to_lowercase() == s_lower {
                Ok(t)
            } else {
                Err(Rich::custom(
                    e.span(),
                    format!("Expected '{}' (case-insensitive)", t),
                ))
            }
        })
}

#[allow(dead_code)]
pub(crate) fn is_ending<'a>()
-> impl Parser<'a, &'a str, Option<String>, extra::Full<Rich<'a, char>, SimpleState<ParserState>, ()>>
+ Clone {
    any()
        .repeated()
        .then(just('\n').map(|c| Some(String::from(c))).or(end().to(None)))
        .map(|_| Some("OK".to_string()))
}

/// 解析零个或多个空白字符（包括空格、制表符等）
pub(crate) fn whitespaces<'a>()
-> impl Parser<'a, &'a str, String, extra::Full<Rich<'a, char>, SimpleState<ParserState>, ()>> + Clone
{
    one_of(" \t").repeated().collect::<String>()
}
/// 解析一个或多个空白字符（包括空格、制表符等）
pub(crate) fn whitespaces_g1<'a>()
-> impl Parser<'a, &'a str, String, extra::Full<Rich<'a, char>, SimpleState<ParserState>, ()>> + Clone
{
    one_of(" \t").repeated().at_least(1).collect::<String>()
}

/// 解析一行:
/// Line <- (!EOL .)+
/// EOL <- '\r'? '\n'
pub(crate) fn line_parser<'a>()
-> impl Parser<'a, &'a str, String, extra::Full<Rich<'a, char>, SimpleState<ParserState>, ()>> + Clone
{
    let end_of_line = one_of("\r")
        .repeated()
        .at_most(1)
        .collect::<String>()
        .then(just("\n"))
        .map(|(s, n)| {
            let mut ans = String::from(s);
            ans.push_str(n);
            ans
        });

    any()
        .and_is(end_of_line.not())
        .repeated()
        .at_least(1)
        .collect::<String>()
        .then(end_of_line)
        .map(|(line, eol)| {
            let mut ans = String::from(line);
            ans.push_str(&eol);
            ans
        })
}

pub(crate) fn blank_line_str_parser<'a>()
-> impl Parser<'a, &'a str, String, extra::Full<Rich<'a, char>, SimpleState<ParserState>, ()>> + Clone
{
    whitespaces()
        .then(one_of("\r").repeated().at_most(1).collect::<String>())
        .then(just("\n"))
        .map(|((ws, cr), nl)| {
            let mut text = String::new();

            if ws.len() > 0 {
                text.push_str(&ws);
            }

            if cr.len() > 0 {
                text.push_str(&cr);
            }

            text.push_str(nl);

            text
        })
}

/// Blank Line Parser := 空白字符后紧跟行终止符, PEG定义如下
/// ```text
/// BlankLine <- WS* EOL
/// WS <- [ \t]
/// EOL <- '\r'? '\n'
/// ```
pub(crate) fn blank_line_parser<'a>()
-> impl Parser<'a, &'a str, GreenToken, extra::Full<Rich<'a, char>, SimpleState<ParserState>, ()>>
+ Clone {
    whitespaces()
        .then(one_of("\r").repeated().at_most(1).collect::<String>())
        .then(just("\n"))
        .map(|((ws, cr), nl)| {
            let mut text = String::new();

            if ws.len() > 0 {
                text.push_str(&ws);
            }

            if cr.len() > 0 {
                text.push_str(&cr);
            }

            text.push_str(nl);

            GreenToken::new(OrgSyntaxKind::BlankLine.into(), &text)
        })
}

/// Text Parser
pub(crate) fn text_parser<'a>()
-> impl Parser<'a, &'a str, S2, extra::Full<Rich<'a, char>, SimpleState<ParserState>, ()>> + Clone {
    any()
        .and_is(text_markup_parser().not())
        .and_is(entity_parser().not())
        .and_is(link_parser().not())
        .and_is(latex_fragment_parser().not())
        .and_is(footnote_reference_parser().not())
        .repeated()
        .at_least(1)
        .collect::<String>()
        .map(|s| {
            S2::Single(NodeOrToken::<GreenNode, GreenToken>::Token(
                GreenToken::new(OrgSyntaxKind::Text.into(), &s),
            ))
        })
}

/// Link parser
pub(crate) fn link_parser<'a>()
-> impl Parser<'a, &'a str, S2, extra::Full<Rich<'a, char>, SimpleState<ParserState>, ()>> + Clone {
    just("[")
        .then(
            just("[")
                .then(none_of("]").repeated().collect::<String>())
                .then(just("]"))
                .map(|((lbracket, path), rbracket)| {
                    let mut children = vec![];
                    children.push(NodeOrToken::Token(GreenToken::new(
                        OrgSyntaxKind::LeftSquareBracket.into(),
                        lbracket,
                    )));

                    children.push(NodeOrToken::Token(GreenToken::new(
                        OrgSyntaxKind::Text.into(),
                        &path,
                    )));

                    children.push(NodeOrToken::Token(GreenToken::new(
                        OrgSyntaxKind::RightSquareBracket.into(),
                        rbracket,
                    )));
                    NodeOrToken::<GreenNode, GreenToken>::Node(GreenNode::new(
                        OrgSyntaxKind::LinkPath.into(),
                        children,
                    ))
                }),
        )
        .then(
            just("[")
                .then(none_of("]").repeated().collect::<String>())
                .then(just("]"))
                .or_not()
                .map(|description| match description {
                    None => None,

                    Some(((lbracket, content), rbracket)) => {
                        let mut children = vec![];
                        children.push(NodeOrToken::Token(GreenToken::new(
                            OrgSyntaxKind::LeftSquareBracket.into(),
                            lbracket,
                        )));

                        children.push(NodeOrToken::Token(GreenToken::new(
                            OrgSyntaxKind::Text.into(),
                            &content,
                        )));

                        children.push(NodeOrToken::Token(GreenToken::new(
                            OrgSyntaxKind::RightSquareBracket.into(),
                            rbracket,
                        )));

                        Some(NodeOrToken::<GreenNode, GreenToken>::Node(GreenNode::new(
                            OrgSyntaxKind::LinkDescription.into(),
                            children,
                        )))
                    }
                }),
        )
        .then(just("]"))
        .map(|(((lbracket, path), maybe_desc), rbracket)| {
            let mut children = vec![];

            children.push(NodeOrToken::Token(GreenToken::new(
                OrgSyntaxKind::LeftSquareBracket.into(),
                lbracket,
            )));

            children.push(path);

            match maybe_desc {
                None => {}
                Some(desc) => {
                    children.push(desc);
                }
            }

            children.push(NodeOrToken::Token(GreenToken::new(
                OrgSyntaxKind::RightSquareBracket.into(),
                rbracket,
            )));

            let link = NodeOrToken::<GreenNode, GreenToken>::Node(GreenNode::new(
                OrgSyntaxKind::Link.into(),
                children,
            ));

            S2::Single(link)
        })
}

/// Footntoe refrence
// fixme: only one pattern suppoted
// - [fn:LABEL] done
// - [fn:LABEL:DEFINITION] todo
// - [fn::DEFINITION] todo

pub(crate) fn footnote_reference_parser<'a>()
-> impl Parser<'a, &'a str, S2, extra::Full<Rich<'a, char>, SimpleState<ParserState>, ()>> + Clone {
    let label = any()
        .filter(|c: &char| c.is_ascii_alphanumeric() || matches!(c, '_' | '-'))
        .repeated()
        .at_least(1)
        .collect::<String>();

    // let definition = object_parser(); // make object_parser: recursive
    // FIXME: simplified version
    let definition = any().and_is(just("]").not()).repeated().collect::<String>();

    // [fn:LABEL:DEFINITION]
    let t2 = just("[fn:")
        .then(label)
        .then(just(":"))
        .then(definition)
        .then(just("]"))
        .map(|((((left_fn_c, label), colon), definition), rbracket)| {
            let mut children = vec![];

            children.push(NodeOrToken::Token(GreenToken::new(
                OrgSyntaxKind::LeftSquareBracket.into(),
                "[",
            )));

            children.push(NodeOrToken::Token(GreenToken::new(
                OrgSyntaxKind::Text.into(),
                "fn",
            )));

            children.push(NodeOrToken::Token(GreenToken::new(
                OrgSyntaxKind::Colon.into(),
                ":",
            )));

            children.push(NodeOrToken::Token(GreenToken::new(
                OrgSyntaxKind::Text.into(),
                &label,
            )));

            children.push(NodeOrToken::Token(GreenToken::new(
                OrgSyntaxKind::Colon.into(),
                ":",
            )));

            children.push(NodeOrToken::Token(GreenToken::new(
                OrgSyntaxKind::Text.into(),
                &definition,
            )));

            children.push(NodeOrToken::Token(GreenToken::new(
                OrgSyntaxKind::RightSquareBracket.into(),
                rbracket,
            )));

            S2::Single(NodeOrToken::Node(GreenNode::new(
                OrgSyntaxKind::FootnoteReference.into(),
                children,
            )))
        });

    // [fn::DEFINITION]
    let t3 = just("[fn::").then(definition).then(just("]")).map(
        |((left_fn_c_c, definition), rbracket)| {
            let mut children = vec![];

            children.push(NodeOrToken::Token(GreenToken::new(
                OrgSyntaxKind::LeftSquareBracket.into(),
                "[",
            )));

            children.push(NodeOrToken::Token(GreenToken::new(
                OrgSyntaxKind::Text.into(),
                "fn",
            )));

            children.push(NodeOrToken::Token(GreenToken::new(
                OrgSyntaxKind::Colon2.into(),
                "::",
            )));

            children.push(NodeOrToken::Token(GreenToken::new(
                OrgSyntaxKind::Text.into(),
                &definition,
            )));

            children.push(NodeOrToken::Token(GreenToken::new(
                OrgSyntaxKind::RightSquareBracket.into(),
                rbracket,
            )));

            S2::Single(NodeOrToken::Node(GreenNode::new(
                OrgSyntaxKind::FootnoteReference.into(),
                children,
            )))
        },
    );

    // [fn:LABEL]
    let t1 = just("[fn:")
        .then(label)
        .then(just("]"))
        .map(|((left_fn_c, label), rbracket)| {
            let mut children = vec![];

            children.push(NodeOrToken::Token(GreenToken::new(
                OrgSyntaxKind::LeftSquareBracket.into(),
                "[",
            )));

            children.push(NodeOrToken::Token(GreenToken::new(
                OrgSyntaxKind::Text.into(),
                "fn",
            )));

            children.push(NodeOrToken::Token(GreenToken::new(
                OrgSyntaxKind::Colon.into(),
                ":",
            )));

            children.push(NodeOrToken::Token(GreenToken::new(
                OrgSyntaxKind::Text.into(),
                &label,
            )));

            children.push(NodeOrToken::Token(GreenToken::new(
                OrgSyntaxKind::RightSquareBracket.into(),
                rbracket,
            )));

            S2::Single(NodeOrToken::Node(GreenNode::new(
                OrgSyntaxKind::FootnoteReference.into(),
                children,
            )))
        });

    // t1
    t1.or(t2).or(t3)
}

// objects_parser
pub(crate) fn object_parser<'a>()
-> impl Parser<'a, &'a str, Vec<S2>, extra::Full<Rich<'a, char>, SimpleState<ParserState>, ()>> + Clone
{
    choice((
        text_markup_parser(),
        entity_parser(),
        link_parser(),
        footnote_reference_parser(),
        latex_fragment_parser(),
        text_parser(),
    ))
    .repeated()
    .at_least(1)
    .collect::<Vec<_>>()
}

// Latex Frament parser
pub(crate) fn latex_fragment_parser<'a>()
-> impl Parser<'a, &'a str, S2, extra::Full<Rich<'a, char>, SimpleState<ParserState>, ()>> + Clone {
    let pre = any().filter(|c| !matches!(c, '$'));
    let border1 = none_of("\r\n \t.,;$");
    let border2 = none_of("\r\n \t.,$");
    let post =
        any().filter(|c: &char| c.is_ascii_punctuation() || matches!(c, ' ' | '\t' | '\r' | '\n'));

    let name = any()
        .filter(|c: &char| c.is_alphabetic())
        .repeated()
        .at_least(1)
        .collect::<String>()
        .filter(|name| !entityname_to_html.contains_key(name));

    // \NAME [CONTENTS1]
    let t01 = just(r##"\"##)
        .then(name)
        .then(just("["))
        .then(
            none_of("{}[]\r\n")
                .and_is(just("]").not())
                .repeated()
                .collect::<String>(),
        )
        .then(just("]"))
        .map(|((((bs, name), lb), content), rb)| {
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
        .map(|((((bs, name), lb), content), rb)| {
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
        .map(|(((((pre, d_pre), border1), body), border2), d_post)| {
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

    // PRE$CHAR$POST
    let t4 = pre
        .then(just("$"))
        .then(none_of(".,?;\" \t"))
        .then(just("$"))
        .then_ignore(post.rewind())
        .map(|(((pre, d_pre), c), d_post)| {
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

    // $$CONTENTS$$
    let t3 = just("$$")
        .then(
            // take_until
            any()
                .and_is(just("$$").not())
                .repeated()
                .collect::<String>(),
        )
        .then(just("$$"))
        .map(|((dd_pre, content), dd_post)| {
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

    // \(CONTENTS\)
    let t1 = just(r##"\("##)
        .then(
            // take_until
            any()
                .and_is(just(r##"\)"##).not())
                .repeated()
                .collect::<String>(),
        )
        .then(just(r##"\)"##))
        .map(|((dd_pre, content), dd_post)| {
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
    let t2 = just(r##"\["##)
        .then(
            // take_until
            any()
                .and_is(just(r##"\]"##).not())
                .repeated()
                .collect::<String>(),
        )
        .then(just(r##"\]"##))
        .map(|((dd_pre, content), dd_post)| {
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

    t1.or(t2).or(t3).or(t4).or(t5).or(t01).or(t02)
}

/// Entity parser
pub(crate) fn entity_parser<'a>()
-> impl Parser<'a, &'a str, S2, extra::Full<Rich<'a, char>, SimpleState<ParserState>, ()>> + Clone {
    let name_parser = any()
        .filter(|c: &char| matches!(c, 'a'..'z' | 'A'..'Z'))
        .repeated()
        .at_least(1)
        .collect::<String>()
        .filter(|name| entityname_to_html.contains_key(name));

    let post_parser = any().filter(|c: &char| !c.is_alphabetic());

    // pattern1: \NAME POST
    let a1 = just(r"\")
        .then(name_parser) // NAME
        .then_ignore(post_parser.rewind()) // POST
        .map(|(backslash, name)| {
            let mut children = vec![];
            children.push(NodeOrToken::Token(GreenToken::new(
                OrgSyntaxKind::BackSlash.into(),
                backslash,
            )));
            children.push(NodeOrToken::Token(GreenToken::new(
                OrgSyntaxKind::EntityName.into(),
                &name,
            )));

            S2::Single(NodeOrToken::Node(GreenNode::new(
                OrgSyntaxKind::Entity.into(),
                children,
            )))
        });

    // Pattern2: \NAME{}
    let a2 = just(r"\").then(name_parser).then(just("{}")).map(
        |((backslash, name), left_right_curly)| {
            let mut children = vec![];
            children.push(NodeOrToken::Token(GreenToken::new(
                OrgSyntaxKind::BackSlash.into(),
                backslash,
            )));
            children.push(NodeOrToken::Token(GreenToken::new(
                OrgSyntaxKind::EntityName.into(),
                &name,
            )));
            children.push(NodeOrToken::Token(GreenToken::new(
                OrgSyntaxKind::LeftCurlyBracket.into(),
                &left_right_curly[0..1],
            )));
            children.push(NodeOrToken::Token(GreenToken::new(
                OrgSyntaxKind::RightCurlyBracket.into(),
                &left_right_curly[1..2],
            )));

            S2::Single(NodeOrToken::Node(GreenNode::new(
                OrgSyntaxKind::Entity.into(),
                children,
            )))
        },
    );

    // pattern3:  \_SPACES
    let a3 = just(r"\")
        .then(just("_"))
        .then(
            one_of(" ")
                .repeated()
                .at_least(1)
                .at_most(20)
                .collect::<String>(),
        )
        .map(|((backslash, us), ws)| {
            let mut children = vec![];
            children.push(NodeOrToken::Token(GreenToken::new(
                OrgSyntaxKind::BackSlash.into(),
                backslash,
            )));

            // content
            children.push(NodeOrToken::Token(GreenToken::new(
                OrgSyntaxKind::UnderScore.into(),
                us,
            )));

            children.push(NodeOrToken::Token(GreenToken::new(
                OrgSyntaxKind::Spaces.into(),
                &ws,
            )));

            S2::Single(NodeOrToken::Node(GreenNode::new(
                OrgSyntaxKind::Entity.into(),
                children,
            )))
        });

    // priority: a2 > a1, or
    // - a1: \pi{} -> Entity(\pi) + Text({})
    // - a2: \pi{} -> Entity(\pi{})
    a2.or(a1).or(a3)
}

fn is_all_whitespace(s: String) -> bool {
    for c in s.chars() {
        if !matches!(c, '\t' | ' ' | '​') {
            return false;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::syntax::OrgLanguage;
    use rowan::SyntaxNode;

    #[test]
    fn test_is_ending() {
        let mut state = SimpleState(ParserState::default());
        for input in vec![" \n", "\t\n", "\n", " \t   \n", " ", "\t", "abc"] {
            assert_eq!(
                is_ending()
                    .parse_with_state(input, &mut state)
                    .into_result(),
                Ok(Some("OK".to_string()))
            );
        }
    }

    #[test]
    fn test_blank_line() {
        let mut state = SimpleState(ParserState::default());
        for input in vec![" \n", "\t\n", "\n", " \t   \n"] {
            assert_eq!(
                blank_line_parser()
                    .parse_with_state(input, &mut state)
                    .into_result(),
                Ok(GreenToken::new(OrgSyntaxKind::BlankLine.into(), input))
            );
        }

        for input in vec![" \n "] {
            assert_eq!(
                blank_line_parser()
                    .parse_with_state(input, &mut state)
                    .has_errors(),
                true
            );
        }
    }
    #[test]
    fn test_line() {
        let mut state = SimpleState(ParserState::default());
        let input = "a row\n";
        let s = line_parser().parse_with_state(input, &mut state);
        println!("{:?}", s);
    }

    #[test]
    fn test_line_lookahead() {
        let mut state = SimpleState(ParserState::default());
        let input = r##"L1
L2
L3

"##;

        // How to debug?
        // x.repeated().collect().then_ignore(y.rewind().not()) BAD
        // x.then_ignore(y.rewind().not()).repeated().collect() OK
        // L1 L2 L3 BL
        let parser = line_parser()
            .then_ignore(blank_line_parser().rewind().not())
            .repeated()
            .collect::<Vec<String>>()
            .then(line_parser())
            .then(blank_line_parser())
            .then(end())
            .map(|s| {
                // println!("s={:?}", s);
                Some(1u32)
            });

        // collect()后似乎不能回退!!
        let parser_bad = line_parser()
            .repeated()
            .collect::<Vec<String>>()
            .then_ignore(blank_line_parser().rewind().not())
            .then(any().repeated())
            .then(end())
            .map(|s| {
                // println!("s={:?}", s);
                Some(1u32)
            });

        // println!("input={:?}", input);
        // let s = parser.lazy().parse_with_state(input, & mut state);
        // println!("{:?}, has_output={:?}, has_errors={:?}", s, s.has_output(), s.has_errors());

        println!("input={:?}", input);
        let s = parser_bad.lazy().parse_with_state(input, &mut state);
        println!(
            "{:?}, has_output={:?}, has_errors={:?}",
            s,
            s.has_output(),
            s.has_errors()
        );
    }

    #[test]
    fn test_correct_entity() {
        let input = vec![
            // pattern1
            "\\alpha ",
            "\\alpha\n",
            // pattern2
            "\\alpha{}",
            // pattern3
            "\\_ \n",
            "\\_  \n",
            "\\_                       \n",
        ];
        let parser = object_parser();
        for e in input {
            let s = parser.parse(e);
            let s1 = s.output().unwrap().iter().next();

            match s1 {
                Some(S2::Single(node)) => {
                    let kind = node.kind();
                    assert_eq!(kind, OrgSyntaxKind::Entity.into());
                }
                _ => {}
            }

            println!(
                "{:?}, has_output={:?}, has_errors={:?}",
                s,
                s.has_output(),
                s.has_errors()
            );
        }
    }

    #[test]
    fn test_incorrect_entity() {
        let input = vec!["\\alphA ", "\\deltab "];
        let parser = object_parser();
        for e in input {
            let s = parser.parse(e);
            let s1 = s.output().unwrap().iter().next();

            match s1 {
                Some(S2::Single(node)) => {
                    let kind = node.kind();
                    assert_ne!(kind, OrgSyntaxKind::Entity.into());
                }
                _ => {}
            }

            // println!(
            //     "{:?}, has_output={:?}, has_errors={:?}",
            //     s,
            //     s.has_output(),
            //     s.has_errors()
            // );
        }
    }

    #[test]
    fn test_link() {
        let input = "[[https://www.baidu.com][baidu]]";

        let parser = link_parser();

        match parser.parse(input).unwrap() {
            S2::Single(node) => {
                let syntax_tree: SyntaxNode<OrgLanguage> =
                    SyntaxNode::new_root(node.into_node().expect("xxx"));
                println!("{:#?}", syntax_tree);

                assert_eq!(
                    format!("{syntax_tree:#?}"),
                    r###"Link@0..32
  LeftSquareBracket@0..1 "["
  LinkPath@1..24
    LeftSquareBracket@1..2 "["
    Text@2..23 "https://www.baidu.com"
    RightSquareBracket@23..24 "]"
  LinkDescription@24..31
    LeftSquareBracket@24..25 "["
    Text@25..30 "baidu"
    RightSquareBracket@30..31 "]"
  RightSquareBracket@31..32 "]"
"###
                );
            }

            _ => {}
        };
    }

    #[test]
    fn test_object() {
        // let input = "[[https://www.baidu.com][baidu]]";
        let input = "foo [[https://www.baidu.com][baidu]]";

        for e in object_parser().parse(input).unwrap() {
            match e {
                S2::Single(node_or_token) => {
                    match node_or_token {
                        NodeOrToken::Node(node) => {
                            let syntax_tree: SyntaxNode<OrgLanguage> = SyntaxNode::new_root(node);
                            println!("{:#?}", syntax_tree);
                        }

                        NodeOrToken::Token(token) => {
                            println!("{:#?}", token);
                        }

                        _ => {}
                    }
                    // println!("{:?}", node);
                    // let syntax_tree: SyntaxNode<OrgLanguage> = SyntaxNode::new_root(node.into_node().expect("xxx"));
                    // println!("{:#?}", syntax_tree);
                }
                _ => {}
            };
        }
    }
}

// block_parser
//   source_block_parser
//   center_block_parser
//   quote_block_parser
// drawer_parser
// dynmic_block_parser
// footnote_definition_parser
// inline_task?
// list_parser
//   items?
//   plain_list_parser: recusive?
// table_parser

// whitespace_config?
