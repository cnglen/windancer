//! Entity parser
use crate::parser::ParserState;
use crate::parser::S2;
use crate::parser::syntax::OrgSyntaxKind;

use chumsky::inspector::Inspector;
use chumsky::inspector::RollbackState;
use chumsky::prelude::*;
use rowan::{GreenNode, GreenToken, NodeOrToken};

use phf::phf_map;

type NT = NodeOrToken<GreenNode, GreenToken>;
type OSK = OrgSyntaxKind;

pub(crate) static ENTITYNAME_TO_HTML: phf::Map<&'static str, &'static str> = phf_map! {
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

/// Entity parser
pub(crate) fn entity_parser<'a>()
-> impl Parser<'a, &'a str, S2, extra::Full<Rich<'a, char>, RollbackState<ParserState>, ()>> + Clone
{
    let name_parser = any()
        .filter(|c: &char| matches!(c, 'a'..'z' | 'A'..'Z'| '0'..'9'))
        .repeated()
        .at_least(1)
        .collect::<String>()
        .filter(|name| ENTITYNAME_TO_HTML.contains_key(name));

    let post_parser = any()
        .filter(|c: &char| !c.is_alphabetic())
        .or(end().to('x'));

    // pattern1: \NAME POST
    let a1 = just::<_, _, extra::Full<Rich<'_, char>, RollbackState<ParserState>, ()>>(r"\")
        .then(name_parser) // NAME
        .then_ignore(post_parser.rewind()) // POST
        .map_with(|(backslash, name), e| {
            e.state().prev_char = name.chars().last();
            let mut children = vec![];
            children.push(NT::Token(GreenToken::new(OSK::BackSlash.into(), backslash)));
            children.push(NT::Token(GreenToken::new(OSK::EntityName.into(), &name)));

            S2::Single(NT::Node(GreenNode::new(OSK::Entity.into(), children)))
        });

    // Pattern2: \NAME{}
    let a2 = just(r"\").then(name_parser).then(just("{}")).map_with(
        |((backslash, name), left_right_curly), e| {
            e.state().prev_char = left_right_curly.chars().last();
            let mut children = vec![];
            children.push(NT::Token(GreenToken::new(OSK::BackSlash.into(), backslash)));
            children.push(NT::Token(GreenToken::new(OSK::EntityName.into(), &name)));
            children.push(NT::Token(GreenToken::new(
                OSK::LeftCurlyBracket.into(),
                &left_right_curly[0..1],
            )));
            children.push(NT::Token(GreenToken::new(
                OSK::RightCurlyBracket.into(),
                &left_right_curly[1..2],
            )));

            S2::Single(NT::Node(GreenNode::new(OSK::Entity.into(), children)))
        },
    );

    // pattern3:  \_SPACES
    let a3 = just::<_, _, extra::Full<Rich<'_, char>, RollbackState<ParserState>, ()>>(r"\")
        .then(just("_"))
        .then(
            one_of(" ")
                .repeated()
                .at_least(1)
                .at_most(20)
                .collect::<String>(),
        )
        .map_with(|((backslash, us), ws), e| {
            e.state().prev_char = ws.chars().last();
            let mut children = vec![];
            children.push(NT::Token(GreenToken::new(OSK::BackSlash.into(), backslash)));
            children.push(NT::Token(GreenToken::new(OSK::Underscore.into(), us)));
            children.push(NT::Token(GreenToken::new(OSK::Spaces.into(), &ws)));

            S2::Single(NT::Node(GreenNode::new(OSK::Entity.into(), children)))
        });

    // priority: `a2` > `a1` since `a2` is longer and includes `a1`, or "\pi{}" will be parsed into <Entity(\pi) + Text({})>, while <Entity(\pi{})> is expected
    a2.or(a1).or(a3)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::common::{get_parser_output, get_parsers_output};
    use crate::parser::object;
    use pretty_assertions::assert_eq;

    #[test]
    fn test_01_name_post() {
        // \NAME POST(where POST=EOF)
        assert_eq!(
            get_parser_output(entity_parser(), r"\alpha"),
            r###"Entity@0..6
  BackSlash@0..1 "\\"
  EntityName@1..6 "alpha"
"###
        );

        assert_eq!(
            get_parsers_output(object::objects_parser(), r"\alpha foo \beta"),
            format!(
                r###"Root@0..16
  Entity@0..6
    BackSlash@0..1 "\\"
    EntityName@1..6 "alpha"
  Text@6..11 " foo "
  Entity@11..16
    BackSlash@11..12 "\\"
    EntityName@12..16 "beta"
"###
            )
        );

        assert_eq!(
            get_parsers_output(
                object::objects_parser(),
                r"\alpha
"
            ),
            format!(
                r###"Root@0..7
  Entity@0..6
    BackSlash@0..1 "\\"
    EntityName@1..6 "alpha"
  Text@6..7 "\n"
"###
            )
        );

        assert_eq!(
            get_parsers_output(object::objects_parser(), r"\alphafoo"),
            format!(
                r###"Root@0..9
  Text@0..9 "\\alphafoo"
"###
            )
        );
    }

    #[test]
    fn test_02_name_curly_bracket() {
        // \NAME{}
        assert_eq!(
            get_parser_output(entity_parser(), r"\beta{}"),
            r###"Entity@0..7
  BackSlash@0..1 "\\"
  EntityName@1..5 "beta"
  LeftCurlyBracket@5..6 "{"
  RightCurlyBracket@6..7 "}"
"###
        );

        assert_eq!(
            get_parsers_output(object::objects_parser(), r"\pi{}d"),
            r###"Root@0..6
  Entity@0..5
    BackSlash@0..1 "\\"
    EntityName@1..3 "pi"
    LeftCurlyBracket@3..4 "{"
    RightCurlyBracket@4..5 "}"
  Text@5..6 "d"
"###
        );

        assert_eq!(
            get_parsers_output(object::objects_parser(), r"\pid"),
            r###"Root@0..4
  Text@0..4 "\\pid"
"###
        );
    }

    #[test]
    fn test_03_spaces() {
        // \_SPACES
        assert_eq!(
            get_parser_output(entity_parser(), r"\_ "),
            r###"Entity@0..3
  BackSlash@0..1 "\\"
  Underscore@1..2 "_"
  Spaces@2..3 " "
"###
        );
        assert_eq!(
            get_parser_output(entity_parser(), r"\_          "),
            r###"Entity@0..12
  BackSlash@0..1 "\\"
  Underscore@1..2 "_"
  Spaces@2..12 "          "
"###
        );

        assert_eq!(
            get_parsers_output(object::objects_parser(), r"\_          \n"),
            r###"Root@0..14
  Entity@0..12
    BackSlash@0..1 "\\"
    Underscore@1..2 "_"
    Spaces@2..12 "          "
  Text@12..14 "\\n"
"###
        );
    }

    #[test]
    fn test_04_bad_entity() {
        assert_eq!(
            get_parsers_output(object::objects_parser(), r"\alphA \deltab "),
            r###"Root@0..15
  Text@0..15 "\\alphA \\deltab "
"###
        );
    }
}
