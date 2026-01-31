//     fn build_roam_graph(&mut self) {
//         // 第一遍：建立 roam_id 到 page_id 的映射
//         for (page_id, page) in &self.pages {
//             if let Some(ref roam_id) = page.metadata.roam_id {
//                 self.roam_id_to_page_id.insert(roam_id.clone(), *page_id);
//             }
//         }

//         // 第二遍：解析链接，在图中添加边
//         for (source_page_id, source_page) in &self.pages {
//             for raw_link in &source_page.metadata.raw_links {
//                 if let RawLink::RoamId { id } = raw_link {
//                     if let Some(&target_page_id) = self.roam_id_to_page_id.get(id) {
//                         // 添加一条从源页面指向目标页面的边
//                         self.graph.add_edge(*source_page_id, target_page_id, LinkType::DirectLink);
//                         // 可选：同时添加一条反向边，或将反向链接单独存储为 Backlink 类型
//                     }
//                 }
//             }
//         }
//     }
// }
// // 链接类型，可用于在图中区分不同关系
// #[derive(Debug, Clone)]
// pub enum LinkType {
//     DirectLink,    // 明确的双向链接
//     Backlink,      // 反向链接（可自动推导）
//     Mention,       // 提及（可能通过文本分析得到）
// }

// // 预计算的相关页面信息
// pub struct RelatedPage {
//     pub page_id: PageId,
//     pub link_type: LinkType,
//     pub snippet: Option<String>, // 可选的上下文摘要
// }

// impl Site {
//     /// 在构建 Site 后，调用此方法建立标签索引
//     pub fn build_tag_index(&mut self) {
//         self.tag_index.clear();
//         for (page_id, page) in &self.pages {
//             for tag in &page.tags {
//                 self.tag_index
//                     .entry(tag.clone())
//                     .or_insert_with(Vec::new)
//                     .push(*page_id);
//             }
//         }
//         // 对每个标签下的页面列表进行排序（例如按日期）
//         for page_ids in self.tag_index.values_mut() {
//             page_ids.sort_by_key(|&id| {
//                 self.pages.get(&id).and_then(|p| p.metadata.date).unwrap_or_default()
//             });
//         }
//     }

//     /// 根据标签获取相关页面
//     pub fn get_pages_by_tag(&self, tag: &str) -> Option<Vec<&Page>> {
//         self.tag_index.get(tag).map(|ids| {
//             ids.iter().filter_map(|id| self.pages.get(id)).collect()
//         })
//     }

//     /// 生成所有标签的聚合页（可在导出阶段调用）
//     pub fn generate_tag_pages(&self) -> HashMap<String, Page> {
//         let mut tag_pages = HashMap::new();
//         for (tag, page_ids) in &self.tag_index {
//             // 为每个标签创建一个虚拟的“聚合页”
//             let tag_page = Page {
//                 id: PageId(usize::MAX), // 使用特殊ID或专门生成
//                 title: format!("Tag: {}", tag),
//                 relative_url: format!("/tags/{}/", tag),
//                 content: self.render_tag_page(tag, page_ids), // 渲染逻辑
//                 tags: HashSet::new(),
//                 // ... 其他字段
//             };
//             tag_pages.insert(tag.clone(), tag_page);
//         }
//         tag_pages
//     }
// }

// impl Site {


// impl Site {
//     /// 建立扁平化导航顺序（例如，深度优先）
//     pub fn build_flattened_order(&mut self) {
//         self.flattened_order.clear();
//         if let Some(root_id) = self.root_page_id {
//             self.dfs_traverse(root_id);
//             // 基于遍历结果，为每个页面设置 prev_flattened_id 和 next_flattened_id
//             self.set_flattened_navigation();
//         }
//     }

//     fn dfs_traverse(&mut self, current_page_id: PageId) {
//         if let Some(page) = self.pages.get(&current_page_id) {
//             // 1. 首先访问当前页面
//             self.flattened_order.push(current_page_id);
//             // 2. 然后递归访问所有子页面（按 children_ids 顺序）
//             for &child_id in &page.children_ids {
//                 self.dfs_traverse(child_id);
//             }
//             // (如果是后序遍历，则将 `push` 操作移到递归之后)
//         }
//     }

//     fn set_flattened_navigation(&mut self) {
//         // 清空现有关系
//         for page in self.pages.values_mut() {
//             page.next_flattened_id = None;
//             page.prev_flattened_id = None;
//         }
//         // 根据顺序列表设置关系
//         for (i, &page_id) in self.flattened_order.iter().enumerate() {
//             if let Some(page) = self.pages.get_mut(&page_id) {
//                 if i > 0 {
//                     page.prev_flattened_id = Some(self.flattened_order[i - 1]);
//                 }
//                 if i + 1 < self.flattened_order.len() {
//                     page.next_flattened_id = Some(self.flattened_order[i + 1]);
//                 }
//             }
//         }
//     }

//     /// 获取当前页面的“下一篇”（扁平化顺序）
//     pub fn get_next_flattened(&self, page_id: PageId) -> Option<&Page> {
//         self.pages.get(&page_id)
//             .and_then(|p| p.next_flattened_id)
//             .and_then(|id| self.pages.get(&id))
//     }
// }
