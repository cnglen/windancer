impl Site {
    /// 从根 Section 构建 Site
    pub fn build_from_section(root_section: &Section, config: &ExportConfig) -> Self {
        let mut site = Site::new();
        let root_page_id = site.process_section(root_section, None, config);
        site.root_page_id = Some(root_page_id);
        site.establish_sibling_links(); // 建立所有同级页面的 prev/next 关系
        site
    }

    fn process_section(
        &mut self,
        section: &Section,
        parent_page_id: Option<PageId>,
        config: &ExportConfig,
    ) -> PageId {
        // 1. 为当前 Section 创建索引页
        let index_page_id = self.create_index_page(section, parent_page_id, config);

        // 2. 处理该 Section 下的所有 Documents
        let mut child_page_ids = Vec::new();
        for doc in §ion.documents {
            // 跳过已用作索引的文档（如 _index.org）
            if !doc.metadata.is_index {
                let page_id = self.create_page_from_document(doc, index_page_id, config);
                child_page_ids.push(page_id);
            }
        }

        // 3. 递归处理所有子 Section
        for subsection in §ion.subsections {
            let subsection_index_id = self.process_section(subsection, Some(index_page_id), config);
            child_page_ids.push(subsection_index_id);
        }

        // 4. 将子页面ID按指定顺序排序，并关联到父页面
        child_page_ids.sort_by_key(|&id| {
            self.pages.get(&id).map(|p| p.metadata.order.unwrap_or(0.0))
        });
        if let Some(parent_page) = self.pages.get_mut(&index_page_id) {
            parent_page.children_ids = child_page_ids;
        }

        index_page_id
    }

    /// 建立所有页面的同级导航 (prev/next) 关系
    fn establish_sibling_links(&mut self) {
        // 遍历所有页面，根据其父页面的 children_ids 顺序设置 prev/next
        for page in self.pages.values_mut() {
            page.prev_id = None;
            page.next_id = None;
        }

        // 为每个父页面下的子页面链表建立关系
        for parent_page in self.pages.values() {
            let children = &parent_page.children_ids;
            for (i, &child_id) in children.iter().enumerate() {
                if let Some(child_page) = self.pages.get_mut(&child_id) {
                    if i > 0 {
                        child_page.prev_id = Some(children[i - 1]);
                    }
                    if i + 1 < children.len() {
                        child_page.next_id = Some(children[i + 1]);
                    }
                }
            }
        }
    }
}
