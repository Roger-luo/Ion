use std::path::{Path, PathBuf};

use crate::Result;

const IGNORED_DIRS: [&str; 4] = [".git", "node_modules", "target", ".cache"];

pub fn discover_skill_files(root: &Path) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();

    let walker = walkdir::WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_entry(|entry| {
            if entry.depth() == 0 {
                return true;
            }

            if !entry.file_type().is_dir() {
                return true;
            }

            let name = entry.file_name().to_string_lossy();
            !IGNORED_DIRS.iter().any(|ignored| *ignored == name)
        });

    for entry in walker {
        let entry = entry.map_err(|err| {
            crate::Error::Io(err.into_io_error().unwrap_or_else(|| {
                std::io::Error::other("failed to walk skill directories")
            }))
        })?;

        if entry.file_type().is_file() && entry.file_name() == "SKILL.md" {
            files.push(entry.path().to_path_buf());
        }
    }

    files.sort();
    files.dedup();
    Ok(files)
}

#[cfg(test)]
mod tests {
    use super::discover_skill_files;

    fn touch(path: &std::path::Path) {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(path, "---\nname: test\ndescription: x\n---\n").unwrap();
    }

    #[test]
    fn discovers_skill_md_recursively() {
        let root = tempfile::tempdir().unwrap();
        let first = root.path().join("a/SKILL.md");
        let second = root.path().join("b/nested/SKILL.md");
        touch(&first);
        touch(&second);

        let found = discover_skill_files(root.path()).unwrap();

        assert_eq!(found.len(), 2);
        assert!(found.contains(&first));
        assert!(found.contains(&second));
    }

    #[test]
    fn ignores_common_heavy_dirs() {
        let root = tempfile::tempdir().unwrap();
        let good = root.path().join("skills/good/SKILL.md");
        touch(&good);
        touch(&root.path().join(".git/ignored/SKILL.md"));
        touch(&root.path().join("node_modules/pkg/SKILL.md"));
        touch(&root.path().join("target/build/SKILL.md"));
        touch(&root.path().join(".cache/x/SKILL.md"));

        let found = discover_skill_files(root.path()).unwrap();

        assert_eq!(found, vec![good]);
    }

    #[test]
    fn returns_sorted_results() {
        let root = tempfile::tempdir().unwrap();
        let z = root.path().join("z-last/SKILL.md");
        let a = root.path().join("a-first/SKILL.md");
        touch(&z);
        touch(&a);

        let found = discover_skill_files(root.path()).unwrap();

        assert_eq!(found.len(), 2);
        let mut expected = vec![a, z];
        expected.sort();
        assert_eq!(found, expected);
    }
}
