use std::path::{Path, PathBuf};
use std::process::Command;

fn git(repo: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .current_dir(repo)
        .args(args)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

fn commit(repo: &Path, path: &str, text: &str) -> String {
    let dir = repo.join(path);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("SKILL.md"), format!("---\nname: example\ndescription: Example skill for revision isolation.\n---\n\n{text}\n")).unwrap();
    git(repo, &["add", "."]);
    git(repo, &["commit", "-m", text]);
    git(repo, &["rev-parse", "HEAD"])
}

fn upstream(tmp: &Path) -> PathBuf {
    let repo = tmp.join("upstream");
    std::fs::create_dir_all(&repo).unwrap();
    git(&repo, &["init"]);
    git(&repo, &["config", "user.email", "test@example.com"]);
    git(&repo, &["config", "user.name", "Test"]);
    repo
}

fn project(tmp: &Path, name: &str, repo: &Path, rev: Option<&str>, path: &str) -> PathBuf {
    let project = tmp.join(name);
    std::fs::create_dir_all(&project).unwrap();
    let revision = rev.map(|r| format!(", rev = \"{r}\"")).unwrap_or_default();
    std::fs::write(project.join("Ion.toml"), format!("[options.targets]\nclaude = \".claude/skills\"\n[skills]\nexample = {{ type = \"git\", source = \"{}\", path = \"{path}\"{revision} }}\n", repo.display())).unwrap();
    project
}

fn add_other(project: &Path, repo: &Path, rev: &str, path: &str) {
    use std::io::Write;
    writeln!(
        std::fs::OpenOptions::new()
            .append(true)
            .open(project.join("Ion.toml"))
            .unwrap(),
        "other = {{ type = \"git\", source = \"{}\", path = \"{path}\", rev = \"{rev}\" }}",
        repo.display()
    )
    .unwrap();
}

fn ion(tmp: &Path, project: &Path, args: &[&str]) {
    let output = Command::new(env!("CARGO_BIN_EXE_ion"))
        .env("XDG_DATA_HOME", tmp.join("data"))
        .env("XDG_CONFIG_HOME", tmp.join("config"))
        .current_dir(project)
        .args(args)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{:?}: {}\n{}",
        args,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn body(project: &Path) -> String {
    std::fs::read_to_string(project.join(".claude/skills/example/SKILL.md")).unwrap()
}

fn locked(project: &Path) -> ion_skill::lockfile::LockedSkill {
    ion_skill::lockfile::Lockfile::from_file(&project.join("Ion.lock"))
        .unwrap()
        .find("example")
        .unwrap()
        .clone()
}

fn storage(tmp: &Path, repo: &Path) -> (PathBuf, PathBuf) {
    let source = ion_skill::source::SkillSource::new(
        ion_skill::source::SourceType::Git,
        repo.display().to_string(),
    );
    let hash = format!(
        "{:x}",
        ion_skill::installer::hash_simple(&source.git_url().unwrap())
    );
    (
        tmp.join("data/ion/repos").join(&hash),
        tmp.join("data/ion/repos/git").join(&hash),
    )
}

#[test]
fn pinned_projects_keep_distinct_revisions_and_removed_skill_paths() {
    let tmp = tempfile::tempdir().unwrap();
    let repo = upstream(tmp.path());
    let first = commit(&repo, "old-skill", "First revision");
    let a = project(tmp.path(), "a", &repo, Some(&first), "old-skill");
    ion(tmp.path(), &a, &["add"]);
    let original = body(&a);
    let lock_a = std::fs::read(a.join("Ion.lock")).unwrap();
    std::fs::remove_dir_all(repo.join("old-skill")).unwrap();
    let second = commit(&repo, "new-skill", "Second revision");
    let b = project(tmp.path(), "b", &repo, Some(&second), "new-skill");
    ion(tmp.path(), &b, &["add"]);
    assert_eq!(
        body(&a),
        original,
        "installing another revision must preserve existing deployment"
    );
    assert!(body(&b).contains("Second revision"));
    assert_eq!(std::fs::read(a.join("Ion.lock")).unwrap(), lock_a);
    assert_eq!(locked(&a).commit(), Some(first.as_str()));
    assert_eq!(locked(&b).commit(), Some(second.as_str()));
    ion(tmp.path(), &a, &["add"]);
    assert!(body(&b).contains("Second revision"));
}

#[test]
fn update_leaves_other_projects_at_their_locked_revision() {
    let tmp = tempfile::tempdir().unwrap();
    let repo = upstream(tmp.path());
    let first = commit(&repo, ".", "First revision");
    let a = project(tmp.path(), "a", &repo, None, ".");
    let b = project(tmp.path(), "b", &repo, None, ".");
    ion(tmp.path(), &a, &["add"]);
    ion(tmp.path(), &b, &["add"]);
    let original = body(&a);
    let second = commit(&repo, ".", "Second revision");
    ion(tmp.path(), &b, &["update"]);
    assert_eq!(body(&a), original);
    assert!(body(&b).contains("Second revision"));
    assert_eq!(locked(&a).commit(), Some(first.as_str()));
    assert_eq!(locked(&b).commit(), Some(second.as_str()));
    std::fs::remove_file(a.join(".agents/skills/example")).unwrap();
    ion(tmp.path(), &a, &["add"]);
    assert_eq!(body(&a), original, "reinstall must honor lock commit");
    assert!(body(&b).contains("Second revision"));
}

#[test]
fn collection_skills_can_use_different_commits_in_one_project() {
    let tmp = tempfile::tempdir().unwrap();
    let repo = upstream(tmp.path());
    let first = commit(&repo, "skills/old", "First revision");
    std::fs::remove_dir_all(repo.join("skills/old")).unwrap();
    let second = commit(&repo, "skills/new", "Second revision");
    let a = project(tmp.path(), "a", &repo, Some(&first), "skills/old");
    add_other(&a, &repo, &second, "skills/new");
    ion(tmp.path(), &a, &["add"]);
    assert!(body(&a).contains("First revision"));
    assert!(
        std::fs::read_to_string(a.join(".claude/skills/other/SKILL.md"))
            .unwrap()
            .contains("Second revision")
    );
    assert_eq!(locked(&a).commit(), Some(first.as_str()));
    assert_eq!(
        locked(&a).checksum(),
        Some(
            ion_skill::git::checksum_dir(&a.join(".agents/skills/example"))
                .unwrap()
                .as_str()
        )
    );
}

#[test]
fn new_fetches_do_not_mutate_legacy_deployments() {
    let tmp = tempfile::tempdir().unwrap();
    let repo = upstream(tmp.path());
    let first = commit(&repo, ".", "First revision");
    let (legacy, _) = storage(tmp.path(), &repo);
    ion_skill::git::clone_or_fetch(&repo.display().to_string(), &legacy).unwrap();
    let old = project(tmp.path(), "old", &repo, Some(&first), ".");
    std::fs::create_dir_all(old.join(".claude/skills")).unwrap();
    std::os::unix::fs::symlink(&legacy, old.join(".claude/skills/example")).unwrap();
    let second = commit(&repo, ".", "Second revision");
    let new = project(tmp.path(), "new", &repo, Some(&second), ".");
    ion(tmp.path(), &new, &["add"]);
    assert!(body(&old).contains("First revision"));
    assert_eq!(ion_skill::git::head_commit(&legacy).unwrap(), first);
    assert!(body(&new).contains("Second revision"));
}

#[test]
fn cache_gc_removes_both_storage_layouts_only_after_last_collection_skill() {
    let tmp = tempfile::tempdir().unwrap();
    let repo = upstream(tmp.path());
    let first = commit(&repo, ".", "First revision");
    let a = project(tmp.path(), "a", &repo, Some(&first), ".");
    add_other(&a, &repo, &first, ".");
    ion(tmp.path(), &a, &["add"]);
    let (legacy, modern) = storage(tmp.path(), &repo);
    std::fs::create_dir_all(&legacy).unwrap();
    std::fs::write(legacy.join("legacy-marker"), "legacy").unwrap();
    assert!(modern.exists());
    assert!(
        tmp.path().join("data/ion/registry.toml").exists(),
        "registry must follow the repository data location"
    );
    ion(tmp.path(), &a, &["remove", "other", "--yes"]);
    ion(tmp.path(), &a, &["cache", "gc"]);
    assert!(body(&a).contains("First revision"));
    assert!(legacy.exists());
    ion(tmp.path(), &a, &["remove", "example", "--yes"]);
    ion(tmp.path(), &a, &["cache", "gc", "--dry-run"]);
    assert!(modern.exists());
    assert!(legacy.exists());
    ion(tmp.path(), &a, &["cache", "gc", "--json"]);
    assert!(
        !modern.exists(),
        "unused revision storage must be collected"
    );
    assert!(!legacy.exists(), "unused legacy storage must be collected");
}

#[test]
fn branch_pin_survives_an_unpinned_install_between_pinned_installs() {
    let tmp = tempfile::tempdir().unwrap();
    let repo = upstream(tmp.path());
    let first = commit(&repo, ".", "Release revision");
    git(&repo, &["branch", "release"]);
    let second = commit(&repo, ".", "Default revision");
    let a = project(tmp.path(), "a", &repo, Some("release"), ".");
    let b = project(tmp.path(), "b", &repo, None, ".");
    let c = project(tmp.path(), "c", &repo, Some("release"), ".");
    ion(tmp.path(), &a, &["add"]);
    ion(tmp.path(), &b, &["add"]);
    ion(tmp.path(), &c, &["add"]);
    assert_eq!(locked(&a).commit(), Some(first.as_str()));
    assert_eq!(locked(&b).commit(), Some(second.as_str()));
    assert_eq!(locked(&c).commit(), Some(first.as_str()));
    assert!(body(&c).contains("Release revision"));
}
