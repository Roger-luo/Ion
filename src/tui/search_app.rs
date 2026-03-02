use ion_skill::search::SearchResult;

pub struct SearchApp {
    pub results: Vec<SearchResult>,
    pub selected: usize,
    pub scroll_offset: usize,
    pub visible_height: usize,
    pub should_quit: bool,
    pub should_install: bool,
}

impl SearchApp {
    pub fn new(mut results: Vec<SearchResult>) -> Self {
        results.sort_by(|a, b| b.stars.unwrap_or(0).cmp(&a.stars.unwrap_or(0)));
        Self {
            results,
            selected: 0,
            scroll_offset: 0,
            visible_height: 0,
            should_quit: false,
            should_install: false,
        }
    }

    pub fn selected_result(&self) -> Option<&SearchResult> {
        self.results.get(self.selected)
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
        if self.selected + 1 < self.results.len() {
            self.selected += 1;
            if self.visible_height > 0 && self.selected >= self.scroll_offset + self.visible_height {
                self.scroll_offset = self.selected - self.visible_height + 1;
            }
        }
    }

    pub fn owner_of(result: &SearchResult) -> &str {
        result.name.split_once('/').map_or(&result.name, |(owner, _)| owner)
    }
}
