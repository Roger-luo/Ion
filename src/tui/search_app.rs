use ion_skill::search::{owner_repo_of, SearchResult};

/// A row in the left-panel list. Groups of 2+ skills from the same repo get a
/// `RepoHeader` followed by indented `Skill` rows; standalone skills appear as
/// `Skill` rows with no header.
pub enum ListRow {
    /// Header for a multi-skill repo group. Selecting this installs the whole repo.
    RepoHeader {
        owner_repo: String,
        stars: Option<u64>,
        description: String,
        skill_count: usize,
    },
    /// An individual skill. `grouped` is true when it sits under a `RepoHeader`.
    Skill { result_idx: usize, grouped: bool },
}

pub struct SearchApp {
    pub results: Vec<SearchResult>,
    pub rows: Vec<ListRow>,
    pub selected: usize,
    pub scroll_offset: usize,
    pub visible_height: usize,
    pub should_quit: bool,
    pub should_install: bool,
}

impl SearchApp {
    pub fn new(mut results: Vec<SearchResult>) -> Self {
        results.sort_by(|a, b| b.stars.unwrap_or(0).cmp(&a.stars.unwrap_or(0)));
        let rows = build_rows(&results);
        Self {
            results,
            rows,
            selected: 0,
            scroll_offset: 0,
            visible_height: 0,
            should_quit: false,
            should_install: false,
        }
    }

    /// The install source string for the selected row.
    pub fn selected_install_source(&self) -> Option<&str> {
        match self.rows.get(self.selected)? {
            ListRow::Skill { result_idx, .. } => Some(&self.results[*result_idx].source),
            ListRow::RepoHeader { owner_repo, .. } => Some(owner_repo),
        }
    }

    pub fn move_up(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
            if self.selected < self.scroll_offset {
                self.scroll_offset = self.selected;
            }
        }
    }

    pub fn move_down(&mut self) {
        if self.selected + 1 < self.rows.len() {
            self.selected += 1;
            if self.visible_height > 0 && self.selected >= self.scroll_offset + self.visible_height
            {
                self.scroll_offset = self.selected - self.visible_height + 1;
            }
        }
    }
}

/// Build a flat list of rows, grouping results that share the same owner/repo.
fn build_rows(results: &[SearchResult]) -> Vec<ListRow> {
    // Gather groups preserving first-occurrence order.
    let mut groups: Vec<(String, Vec<usize>)> = Vec::new();
    let mut key_to_idx: std::collections::HashMap<String, usize> =
        std::collections::HashMap::new();

    for (i, r) in results.iter().enumerate() {
        let key = owner_repo_of(&r.source).to_string();
        if let Some(&g) = key_to_idx.get(&key) {
            groups[g].1.push(i);
        } else {
            key_to_idx.insert(key.clone(), groups.len());
            groups.push((key, vec![i]));
        }
    }

    let mut rows = Vec::new();
    for (owner_repo, indices) in groups {
        if indices.len() == 1 {
            // Standalone skill — no header
            rows.push(ListRow::Skill {
                result_idx: indices[0],
                grouped: false,
            });
        } else {
            // Multi-skill group: header + indented skills
            let first = &results[indices[0]];
            rows.push(ListRow::RepoHeader {
                owner_repo,
                stars: first.stars,
                description: first.description.clone(),
                skill_count: indices.len(),
            });
            for idx in indices {
                rows.push(ListRow::Skill {
                    result_idx: idx,
                    grouped: true,
                });
            }
        }
    }
    rows
}
