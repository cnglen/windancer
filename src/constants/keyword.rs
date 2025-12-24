use phf::phf_set;

// 4 group without overlap for affliated keyword
// - OPTVALUE_PARSED
// - OPTVALUE_STRING
// - NONVALUE_PARSED
// - NONVALUE_STRING
pub(crate) static ORG_ELEMENT_KEYWORDS_OPTVALUE_PARSED: phf::Set<&'static str> = phf_set! {
    "CAPTION"
};

pub(crate) static ORG_ELEMENT_KEYWORDS_OPTVALUE_STRING: phf::Set<&'static str> = phf_set! {
    "RESULTS"
};
// pub(crate) static ORG_ELEMENT_KEYWORDS_NONVALUE_PARSED: phf::Set<&'static str> = phf_set! {};
pub(crate) static ORG_ELEMENT_KEYWORDS_NONVALUE_STRING: phf::Set<&'static str> = phf_set! {
    "DATA",
    "HEADER",
    "HEADERS",
    "LABEL",
    "NAME",
    "PLOT",
    "RESNAME",
    "RESULT",
    "SOURCE",
    "SRCNAME",
    "TBLNAME"
};
// used for simple_affiliated_keyword_parser: merge ORG_ELEMENT_KEYWORDS_OPTVALUE_PARSED and ORG_ELEMENT_KEYWORDS_OPTVALUE_STRING
pub(crate) static ORG_ELEMENT_KEYWORDS_OPTVALUE: phf::Set<&'static str> = phf_set! {
    "CAPTION", "RESULTS"
};
