/// config for org parser
use std::collections::HashSet;

#[derive(Clone, Debug)]
pub enum OrgUseSubSuperscripts {
    Nil,   // disable subscript and superscript
    Brace, // only _{} is recognized as subscript, only ^{} is recognized as superscript
    True,  // see https://orgmode.org/worg/org-syntax.html
}

#[derive(Clone, Debug)]
pub struct OrgTodoKeywords {
    pub requiring_action: HashSet<String>,
    pub no_further_action: HashSet<String>,
}

impl Default for OrgTodoKeywords {
    fn default() -> Self {
        Self {
            requiring_action: vec!["TODO"].into_iter().map(String::from).collect(),
            no_further_action: vec!["DONE"].into_iter().map(String::from).collect(),
        }
    }
}

impl OrgTodoKeywords {
    pub fn new(requiring_action: HashSet<String>, no_further_action: HashSet<String>) -> Self {
        Self {
            requiring_action,
            no_further_action,
        }
    }
}

// Toggle inclusion of statistics cookies: (‘org-export-with-statistics-cookies’).
#[derive(Clone, Debug)]
pub struct OrgParserConfig {
    pub org_todo_keywords: OrgTodoKeywords,

    /// Different from org.el, this influences the parsing process.
    pub org_use_sub_superscripts: OrgUseSubSuperscripts,

    /// #+KEY[OPTVAL]: VALUE
    /// Only keywords which are a member of `org_element_parsed_keywords`: VALUE can contain objects
    pub org_element_parsed_keywords: HashSet<String>,

    /// #+KEY[OPTVAL]: VALUE    
    /// Only keywords which are a member of org-element-dual-keywords: [OPTVAL] is supported    
    pub org_element_dual_keywords: HashSet<String>,

    pub org_element_affiliated_keywords: HashSet<String>,
}

impl Default for OrgParserConfig {
    fn default() -> Self {
        Self {
            org_todo_keywords: OrgTodoKeywords::default(),

            org_use_sub_superscripts: OrgUseSubSuperscripts::True,

            // default set to Brace to remove ambiguity
            // org_use_sub_superscripts: OrgUseSubSuperscripts::Brace,
            // org_use_sub_superscripts: OrgUseSubSuperscripts::Nil,
            org_element_parsed_keywords: ["CAPTION"].into_iter().map(String::from).collect(),

            org_element_dual_keywords: ["CAPTION", "RESULTS"]
                .into_iter()
                .map(String::from)
                .collect(),

            org_element_affiliated_keywords: [
                "CAPTION", "DATA", "HEADER", "HEADERS", "LABEL", "NAME", "PLOT", "RESNAME",
                "RESULT", "RESULTS", "SOURCE", "SRCNAME", "TBLNAME",
            ]
            .into_iter()
            .map(String::from)
            .collect(),
        }
    }
}

impl OrgParserConfig {
    // dual keywords which contains objects
    pub(crate) fn org_element_dual_keywords_parsed(&self) -> HashSet<String> {
        self.org_element_dual_keywords
            .intersection(&self.org_element_parsed_keywords)
            .map(|s| s.clone())
            .collect()
    }

    // dual keywords which doesn't contain objects
    pub(crate) fn org_element_dual_keywords_string(&self) -> HashSet<String> {
        self.org_element_dual_keywords
            .difference(&self.org_element_parsed_keywords)
            .map(|s| s.clone())
            .collect()
    }

    // non-dual keywords which contains objects: commented since this is a empty set
    // pub(crate) fn org_element_affiliated_keywords_nondual_parsed(&self) -> HashSet<String> {
    //     self.org_element_affiliated_keywords.iter()
    //         .filter(|s| {
    //             !self.org_element_dual_keywords.contains(*s)
    //                 && self.org_element_parsed_keywords.contains(*s)
    //         })
    //         .map(|s| s.clone())
    //         .collect()
    // }

    // non-dual keywords which doen's contain objects
    pub(crate) fn org_element_affiliated_keywords_nondual_string(&self) -> HashSet<String> {
        self.org_element_affiliated_keywords
            .iter()
            .filter(|s| {
                !self.org_element_dual_keywords.contains(*s)
                    && !self.org_element_parsed_keywords.contains(*s)
            })
            .map(|s| s.clone())
            .collect()
    }
}
