use std::collections::HashMap;

use crate::export::ssg::site::{Page, PageId};

#[derive(serde::Serialize)]
pub struct TableViewModel {
    pub table_number: Option<usize>,
    pub has_caption: bool,
    pub caption: String,
    pub has_header: bool,
    pub header_rows: Vec<String>,
    pub body_rows: Vec<String>,
}

#[derive(serde::Serialize)]
pub struct PageNavContext {
    pub prev_sibling: Option<String>,
    pub next_sibling: Option<String>,
    pub prev_flattened: Option<String>,
    pub next_flattened: Option<String>,
    pub parent: Option<String>,
    pub children: Vec<String>,
    pub nav_valid: bool,
}

impl PageNavContext {
    pub fn from_page(page: &Page, pageid_url: &HashMap<PageId, String>) -> Self {
        let mut nav_valid = false;

        let parent = if let Some(parent_id) = &page.parent_id {
            nav_valid = true;
            Some(pageid_url.get(parent_id).unwrap().to_string())
        } else {
            None
        };

        let children: Vec<String> = page
            .children_ids
            .iter()
            .map(|id| pageid_url.get(id).unwrap().to_string())
            .collect();

        let prev_sibling = if let Some(prev_sibling_id) = &page.prev_sibling_id {
            Some(pageid_url.get(prev_sibling_id).unwrap().to_string())
        } else {
            None
        };

        let next_sibling = if let Some(next_sibling_id) = &page.next_sibling_id {
            Some(pageid_url.get(next_sibling_id).unwrap().to_string())
        } else {
            None
        };

        let prev_flattened = if let Some(prev_flattened_id) = &page.prev_flattened_id {
            nav_valid = true;
            Some(pageid_url.get(prev_flattened_id).unwrap().to_string())
        } else {
            None
        };

        let next_flattened = if let Some(next_flattened_id) = &page.next_flattened_id {
            nav_valid = true;
            Some(pageid_url.get(next_flattened_id).unwrap().to_string())
        } else {
            None
        };

        Self {
            parent,
            children,
            prev_sibling,
            next_sibling,
            prev_flattened,
            next_flattened,
            nav_valid,
        }
    }
}
