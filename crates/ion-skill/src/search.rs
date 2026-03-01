/// Search result from any source.
#[derive(Debug, Clone)]
pub struct SearchResult {
    pub name: String,
    pub description: String,
    pub source: String,
    pub registry: String,
}

/// A searchable source of skills.
pub trait SearchSource {
    fn name(&self) -> &str;
    fn search(&self, query: &str, limit: usize) -> crate::Result<Vec<SearchResult>>;
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeSource {
        results: Vec<SearchResult>,
    }

    impl SearchSource for FakeSource {
        fn name(&self) -> &str {
            "fake"
        }
        fn search(&self, _query: &str, limit: usize) -> crate::Result<Vec<SearchResult>> {
            Ok(self.results.iter().take(limit).cloned().collect())
        }
    }

    #[test]
    fn trait_search_returns_results() {
        let source = FakeSource {
            results: vec![SearchResult {
                name: "test-skill".to_string(),
                description: "A test".to_string(),
                source: "owner/repo/test-skill".to_string(),
                registry: "fake".to_string(),
            }],
        };
        let results = source.search("test", 10).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "test-skill");
    }

    #[test]
    fn trait_search_respects_limit() {
        let source = FakeSource {
            results: vec![
                SearchResult {
                    name: "a".to_string(),
                    description: "".to_string(),
                    source: "".to_string(),
                    registry: "fake".to_string(),
                },
                SearchResult {
                    name: "b".to_string(),
                    description: "".to_string(),
                    source: "".to_string(),
                    registry: "fake".to_string(),
                },
                SearchResult {
                    name: "c".to_string(),
                    description: "".to_string(),
                    source: "".to_string(),
                    registry: "fake".to_string(),
                },
            ],
        };
        let results = source.search("x", 2).unwrap();
        assert_eq!(results.len(), 2);
    }
}
