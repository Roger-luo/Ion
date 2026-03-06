# Local Skills Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add a `local` source type so users can create project-specific skills and eject remote skills into editable local copies.

**Architecture:** New `SourceType::Local` variant with `forked_from` tracking, `skills-dir` config in `ManifestOptions`, enhanced `ion skill new` for local creation, and new `ion skill eject` command. Local skills skip fetch/validation/gitignore — they're tracked by git directly.

**Tech Stack:** Rust, clap, toml_edit, existing `ion-skill` crate infrastructure.

---

### Task 1: Add `Local` variant to `SourceType`

**Files:**
- Modify: `crates/ion-skill/src/source.rs`

**Step 1: Add the Local variant**

In `source.rs`, add `Local` to the `SourceType` enum:

```rust
pub enum SourceType {
    Github,
    Git,
    Http,
    Path,
    Binary,
    Local,
}
```

**Step 2: Add `forked_from` to `SkillSource`**

```rust
pub struct SkillSource {
    pub source_type: SourceType,
    pub source: String,
    pub path: Option<String>,
    pub rev: Option<String>,
    pub version: Option<String>,
    pub binary: Option<String>,
    pub asset_pattern: Option<String>,
    pub forked_from: Option<String>,
}
```

Update all existing `SkillSource` construction sites (in `infer()` and the test in `test_binary_source_type_serializes`) to include `forked_from: None`.

**Step 3: Handle `Local` in `git_url()`**

Add `Local` to the error arm alongside `Path`:

```rust
SourceType::Path | SourceType::Http | SourceType::Local => Err(Error::Source(...))
```

**Step 4: Add a unit test**

```rust
#[test]
fn local_source_type_serializes() {
    let source = SkillSource {
        source_type: SourceType::Local,
        source: String::new(),
        path: None,
        rev: None,
        version: None,
        binary: None,
        asset_pattern: None,
        forked_from: Some("anthropics/skills/brainstorming".to_string()),
    };
    assert_eq!(source.source_type, SourceType::Local);
    assert_eq!(source.forked_from.as_deref(), Some("anthropics/skills/brainstorming"));
}
```

**Step 5: Run tests**

Run: `cargo test -p ion-skill`
Expected: All pass, including the new test.

**Step 6: Commit**

```bash
git add crates/ion-skill/src/source.rs
git commit -m "feat: add Local variant to SourceType with forked_from field"
```

---

### Task 2: Add `skills-dir` to manifest and `local` SkillEntry parsing

**Files:**
- Modify: `crates/ion-skill/src/manifest.rs`

**Step 1: Add `skills_dir` to `ManifestOptions`**

```rust
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct ManifestOptions {
    #[serde(default)]
    pub targets: BTreeMap<String, String>,
    #[serde(default)]
    pub skills_dir: Option<String>,
}
```

Update `get_value()` and `list_values()` to handle `skills-dir`:

```rust
pub fn get_value(&self, key: &str) -> Option<String> {
    if key == "skills-dir" {
        return self.skills_dir.clone();
    }
    let (section, field) = key.split_once('.')?;
    match section {
        "targets" => self.targets.get(field).cloned(),
        _ => None,
    }
}

pub fn list_values(&self) -> Vec<(String, String)> {
    let mut values: Vec<(String, String)> = self.targets
        .iter()
        .map(|(k, v)| (format!("targets.{k}"), v.clone()))
        .collect();
    if let Some(ref sd) = self.skills_dir {
        values.push(("skills-dir".to_string(), sd.clone()));
    }
    values
}
```

**Step 2: Add `forked_from` to `SkillEntry::Full`**

```rust
Full {
    #[serde(rename = "type", default)]
    source_type: Option<SourceType>,
    #[serde(default)]
    source: Option<String>,
    #[serde(default)]
    version: Option<String>,
    #[serde(default)]
    rev: Option<String>,
    #[serde(default)]
    path: Option<String>,
    #[serde(default)]
    binary: Option<String>,
    #[serde(default, alias = "asset-pattern")]
    asset_pattern: Option<String>,
    #[serde(default, alias = "forked-from")]
    forked_from: Option<String>,
},
```

Note: `source` becomes `Option<String>` because local skills don't have a source field — they use `{ type = "local" }`.

**Step 3: Update `resolve_entry` for Local type**

In the `Full` match arm, handle the case where `source_type` is `Some(SourceType::Local)`:

```rust
SkillEntry::Full {
    source_type,
    source,
    version,
    rev,
    path,
    binary,
    asset_pattern,
    forked_from,
} => {
    let mut resolved = if let Some(st) = source_type {
        if *st == SourceType::Local {
            SkillSource {
                source_type: SourceType::Local,
                source: source.clone().unwrap_or_default(),
                path: path.clone(),
                rev: None,
                version: None,
                binary: None,
                asset_pattern: None,
                forked_from: forked_from.clone(),
            }
        } else {
            SkillSource {
                source_type: st.clone(),
                source: source.clone().ok_or_else(|| Error::Manifest("source field is required".to_string()))?,
                path: path.clone(),
                rev: None,
                version: None,
                binary: None,
                asset_pattern: None,
                forked_from: None,
            }
        }
    } else {
        let mut s = SkillSource::infer(source.as_deref().ok_or_else(|| Error::Manifest("source field is required".to_string()))?)?;
        s.forked_from = None;
        s
    };
    // ... rest of version/rev/binary setting stays the same
    // Also propagate forked_from for non-local types if present
    resolved.forked_from = forked_from.clone();
    Ok(resolved)
}
```

**Step 4: Add unit tests**

```rust
#[test]
fn parse_local_skill_entry() {
    let toml_str = "[skills]\nmy-deploy = { type = \"local\" }\n";
    let manifest = Manifest::parse(toml_str).unwrap();
    let source = Manifest::resolve_entry(&manifest.skills["my-deploy"]).unwrap();
    assert_eq!(source.source_type, SourceType::Local);
    assert!(source.forked_from.is_none());
}

#[test]
fn parse_local_skill_with_forked_from() {
    let toml_str = "[skills]\nbrainstorming = { type = \"local\", forked-from = \"anthropics/skills/brainstorming\" }\n";
    let manifest = Manifest::parse(toml_str).unwrap();
    let source = Manifest::resolve_entry(&manifest.skills["brainstorming"]).unwrap();
    assert_eq!(source.source_type, SourceType::Local);
    assert_eq!(source.forked_from.as_deref(), Some("anthropics/skills/brainstorming"));
}

#[test]
fn parse_skills_dir_option() {
    let toml_str = "[skills]\n\n[options]\nskills-dir = \"my-skills\"\n";
    let manifest = Manifest::parse(toml_str).unwrap();
    assert_eq!(manifest.options.skills_dir.as_deref(), Some("my-skills"));
}
```

**Step 5: Run tests**

Run: `cargo test -p ion-skill`
Expected: All pass.

**Step 6: Commit**

```bash
git add crates/ion-skill/src/manifest.rs
git commit -m "feat: add skills-dir option and local skill entry parsing to manifest"
```

---

### Task 3: Update `manifest_writer` for Local type and `skills-dir`

**Files:**
- Modify: `crates/ion-skill/src/manifest_writer.rs`

**Step 1: Handle Local in `skill_to_toml()`**

Add a `SourceType::Local` arm and handle `forked_from`:

```rust
SourceType::Local => {
    table.insert("type", "local".into());
}
```

For Local type, skip the `source` field in `skill_to_toml` (local skills don't have a source). The `needs_table` check should always be true for Local. Add `forked_from` serialization:

```rust
if let Some(ref ff) = source.forked_from {
    table.insert("forked-from", ff.as_str().into());
}
```

Update the `needs_table` check to include `forked_from`:

```rust
let needs_table = source.source_type == SourceType::Local
    || source.forked_from.is_some()
    || source.rev.is_some()
    || source.version.is_some()
    || source.path.is_some()
    || source.binary.is_some()
    || source.asset_pattern.is_some();
```

For Local type, don't insert the `source` key:

```rust
if source.source_type != SourceType::Local {
    let source_str = match (&source.source_type, &source.path) {
        (SourceType::Github, Some(path)) => format!("{}/{}", source.source, path),
        _ => source.source.clone(),
    };
    table.insert("source", source_str.into());
}
```

**Step 2: Add `write_skills_dir()` function**

```rust
/// Write the skills-dir value to Ion.toml's [options] section.
pub fn write_skills_dir(manifest_path: &Path, skills_dir: &str) -> Result<String> {
    let content = std::fs::read_to_string(manifest_path)
        .unwrap_or_else(|_| "[skills]\n".to_string());
    let mut doc: DocumentMut = content.parse().map_err(Error::TomlEdit)?;

    if !doc.contains_key("skills") {
        doc["skills"] = Item::Table(Table::new());
    }

    if !doc.contains_key("options") {
        doc["options"] = Item::Table(Table::new());
    }
    let options = doc["options"]
        .as_table_mut()
        .ok_or_else(|| Error::Manifest("[options] is not a table".to_string()))?;

    options["skills-dir"] = value(skills_dir);

    let result = doc.to_string();
    std::fs::write(manifest_path, &result).map_err(Error::Io)?;
    Ok(result)
}
```

**Step 3: Add unit tests**

```rust
#[test]
fn add_local_skill_to_manifest() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("Ion.toml");
    std::fs::write(&path, "[skills]\n").unwrap();

    let source = SkillSource {
        source_type: SourceType::Local,
        source: String::new(),
        path: None,
        rev: None,
        version: None,
        binary: None,
        asset_pattern: None,
        forked_from: None,
    };

    let result = add_skill(&path, "my-deploy", &source).unwrap();
    assert!(result.contains("my-deploy"));
    assert!(result.contains("local"));
    assert!(!result.contains("source"));
}

#[test]
fn add_local_skill_with_forked_from() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("Ion.toml");
    std::fs::write(&path, "[skills]\n").unwrap();

    let source = SkillSource {
        source_type: SourceType::Local,
        source: String::new(),
        path: None,
        rev: None,
        version: None,
        binary: None,
        asset_pattern: None,
        forked_from: Some("anthropics/skills/brainstorming".to_string()),
    };

    let result = add_skill(&path, "brainstorming", &source).unwrap();
    assert!(result.contains("local"));
    assert!(result.contains("forked-from"));
    assert!(result.contains("anthropics/skills/brainstorming"));
}

#[test]
fn write_skills_dir_to_manifest() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("Ion.toml");
    std::fs::write(&path, "[skills]\n").unwrap();

    write_skills_dir(&path, "my-skills").unwrap();

    let content = std::fs::read_to_string(&path).unwrap();
    assert!(content.contains("skills-dir"));
    assert!(content.contains("my-skills"));
}
```

**Step 4: Run tests**

Run: `cargo test -p ion-skill`
Expected: All pass.

**Step 5: Commit**

```bash
git add crates/ion-skill/src/manifest_writer.rs
git commit -m "feat: add local skill and skills-dir support to manifest_writer"
```

---

### Task 4: Update `install.rs` to handle Local skills

**Files:**
- Modify: `src/commands/install.rs`

**Step 1: Skip fetch/validation for Local skills, just ensure symlinks**

In the install-all flow, local skills need different handling. Before the existing validation loop, add early handling for local skills:

```rust
for (name, entry) in &manifest.skills {
    let source = Manifest::resolve_entry(entry)?;

    // Local skills: just ensure symlinks exist, skip fetch/validation
    if source.source_type == SourceType::Local {
        let skills_dir = merged_options.skills_dir.as_deref().unwrap_or(".agents");
        let local_skill_dir = ctx.project_dir.join(skills_dir).join("skills").join(name);
        if !local_skill_dir.exists() {
            println!("  {} {} — {}", p.warn("⚠"), p.bold(name), "local directory missing, skipping");
            continue;
        }
        installer.deploy(name, &local_skill_dir)?;
        println!("  {} {} — local", p.success("✓"), p.bold(name));
        // Build a locked entry with checksum only
        let checksum = ion_skill::git::checksum_dir(&local_skill_dir).ok();
        let locked = ion_skill::lockfile::LockedSkill {
            name: name.clone(),
            source: String::new(),
            path: None,
            version: None,
            commit: None,
            checksum,
            binary: None,
            binary_version: None,
            binary_checksum: None,
        };
        lockfile.upsert(locked);
        continue;
    }

    // ... existing validation logic for remote skills
```

Also update the gitignore skip condition to include Local:

```rust
if !matches!(entry.source.source_type, SourceType::Path | SourceType::Local) {
```

**Step 2: Run tests**

Run: `cargo test`
Expected: All existing tests pass.

**Step 3: Commit**

```bash
git add src/commands/install.rs
git commit -m "feat: handle local skills in install-all flow"
```

---

### Task 5: Update `update.rs` and `remove.rs` for Local skills

**Files:**
- Modify: `src/commands/update.rs`
- Modify: `src/commands/remove.rs`

**Step 1: Skip Local skills in update**

In `update.rs`, add `SourceType::Local` to the skip condition:

```rust
if matches!(source.source_type, SourceType::Path | SourceType::Http | SourceType::Local) {
    continue;
}
```

**Step 2: Handle Local skills in remove**

In `remove.rs`, for Local skills, remove the `.agents/skills/<name>` symlink and target symlinks, but do NOT delete the actual skill directory under `skills-dir`. The `SkillInstaller::uninstall` already removes the symlinks — but for local skills, `.agents/skills/<name>` might be a real directory or a symlink depending on whether `skills-dir` is `.agents` or something else.

When `skills-dir` is `.agents` (default), the local skill lives at `.agents/skills/<name>` as a real directory. In this case, `uninstall` would delete it — we need to prevent that.

Add a check before calling `uninstall`:

```rust
// For local skills, only remove symlinks in targets, not the skill directory itself
let entry_source = Manifest::resolve_entry(entry);
if let Ok(ref source) = entry_source
    && source.source_type == SourceType::Local
{
    // Only remove target symlinks, not the skill directory
    for target_path in merged_options.targets.values() {
        let target_dir = ctx.project_dir.join(target_path).join(skill_name);
        if target_dir.is_symlink() {
            std::fs::remove_file(&target_dir)?;
        }
    }
    // Remove .agents symlink only if it IS a symlink (custom skills-dir)
    let agents_dir = ctx.project_dir.join(".agents").join("skills").join(skill_name);
    if agents_dir.is_symlink() {
        std::fs::remove_file(&agents_dir)?;
    }
    println!("  Removed symlinks for {}", p.info(&format!("{skill_name}")));
    println!("  {}: local skill directory preserved", p.dim("note"));
} else {
    SkillInstaller::new(&ctx.project_dir, &merged_options).uninstall(skill_name)?;
    println!("  Removed from {}", p.info(&format!(".agents/skills/{skill_name}/")));
}
```

Also skip gitignore removal for local skills (they were never gitignored):

```rust
if !matches!(entry_source.as_ref().map(|s| &s.source_type), Ok(SourceType::Local)) {
    ion_skill::gitignore::remove_skill_entries(&ctx.project_dir, skill_name)?;
    println!("  Updated {}", p.dim(".gitignore"));
}
```

**Step 3: Run tests**

Run: `cargo test`
Expected: All pass.

**Step 4: Commit**

```bash
git add src/commands/update.rs src/commands/remove.rs
git commit -m "feat: handle local skills in update and remove commands"
```

---

### Task 6: Enhance `ion skill new` for local skill creation

**Files:**
- Modify: `src/commands/new.rs`
- Modify: `src/main.rs`

**Step 1: Add `--dir` flag to `SkillCommands::New`**

In `main.rs`, add the `--dir` parameter:

```rust
SkillCommands::New {
    path: Option<String>,
    bin: bool,
    collection: bool,
    force: bool,
    #[arg(long)]
    dir: Option<String>,
},
```

Update the dispatch to pass `dir`:

```rust
SkillCommands::New { path, bin, collection, force, dir } => {
    commands::new::run(path.as_deref(), bin, collection, force, dir.as_deref())
}
```

**Step 2: Update `new::run()` signature and add local skill logic**

When `--dir` is provided (or no `--path`), and we're inside a project with an `Ion.toml`, create the skill under `{skills-dir}/skills/{name}/` and register it in `Ion.toml`.

Update the function signature:

```rust
pub fn run(path: Option<&str>, bin: bool, collection: bool, force: bool, dir: Option<&str>) -> anyhow::Result<()> {
```

Add logic after the existing `target_dir` resolution:

```rust
// If --dir is set, persist skills-dir to Ion.toml
if let Some(d) = dir {
    let manifest_path = std::env::current_dir()?.join("Ion.toml");
    if manifest_path.exists() || path.is_none() {
        ion_skill::manifest_writer::write_skills_dir(&manifest_path, d)?;
    }
}

// When --path is NOT set and we're in a project, create under skills-dir
if path.is_none() && !collection {
    let cwd = std::env::current_dir()?;
    let manifest_path = cwd.join("Ion.toml");
    if manifest_path.exists() || dir.is_some() {
        // Prompt for skill name
        let name = prompt_skill_name()?;
        let skills_dir = dir.unwrap_or(".agents");
        let skill_path = cwd.join(skills_dir).join("skills").join(&name);

        if skill_path.exists() && !force {
            anyhow::bail!(
                "Skill directory already exists at {}. Use --force to overwrite.",
                skill_path.display()
            );
        }

        std::fs::create_dir_all(&skill_path)?;

        // Create the skill content (bin or regular)
        create_skill_content(&skill_path, &name, bin, force)?;

        // Register in Ion.toml as local
        let source = ion_skill::source::SkillSource {
            source_type: ion_skill::source::SourceType::Local,
            source: String::new(),
            path: None,
            rev: None,
            version: None,
            binary: None,
            asset_pattern: None,
            forked_from: None,
        };
        ion_skill::manifest_writer::add_skill(&manifest_path, &name, &source)?;

        // Create symlinks
        // ... (deploy via installer)

        println!("Created local skill '{}' in {}", name, skill_path.display());
        println!("  Registered in Ion.toml as {{ type = \"local\" }}");
        return Ok(());
    }
}
```

Add a `prompt_skill_name()` helper that reads from stdin:

```rust
fn prompt_skill_name() -> anyhow::Result<String> {
    use std::io::Write;
    print!("Skill name: ");
    std::io::stdout().flush()?;
    let mut name = String::new();
    std::io::stdin().read_line(&mut name)?;
    let name = name.trim().to_string();
    if name.is_empty() {
        anyhow::bail!("Skill name cannot be empty");
    }
    Ok(slugify(&name))
}
```

Extract the SKILL.md and binary project creation into a reusable `create_skill_content()` function from the existing code in `run()`.

**Step 3: Run tests**

Run: `cargo test`
Expected: All existing tests pass. (Interactive prompt tests can't run in CI, but `--path` tests still work.)

**Step 4: Commit**

```bash
git add src/commands/new.rs src/main.rs
git commit -m "feat: enhance ion skill new with --dir flag and local skill creation"
```

---

### Task 7: Implement `ion skill eject` command

**Files:**
- Create: `src/commands/eject.rs`
- Modify: `src/main.rs`
- Modify: `src/commands/mod.rs`

**Step 1: Add `Eject` variant to `SkillCommands`**

In `main.rs`:

```rust
#[derive(Subcommand)]
enum SkillCommands {
    // ... existing variants ...
    /// Eject a remote skill into an editable local copy
    Eject {
        /// Name of the skill to eject
        name: String,
    },
}
```

Add dispatch:

```rust
SkillCommands::Eject { name } => commands::eject::run(&name),
```

**Step 2: Add `pub mod eject;` to `src/commands/mod.rs`**

**Step 3: Implement `eject.rs`**

```rust
use ion_skill::manifest::Manifest;
use ion_skill::manifest_writer;
use ion_skill::source::SourceType;

use crate::context::ProjectContext;
use crate::style::Paint;

pub fn run(name: &str) -> anyhow::Result<()> {
    let ctx = ProjectContext::load()?;
    let p = Paint::new(&ctx.global_config);
    let manifest = ctx.manifest()?;

    // 1. Verify skill exists and is remote
    let entry = manifest.skills.get(name)
        .ok_or_else(|| anyhow::anyhow!("Skill '{}' not found in Ion.toml", name))?;
    let source = Manifest::resolve_entry(entry)?;

    if matches!(source.source_type, SourceType::Local | SourceType::Path) {
        anyhow::bail!("Skill '{}' is already local", name);
    }

    // 2. Resolve skills-dir
    let merged_options = ctx.merged_options(&manifest);
    let skills_dir = merged_options.skills_dir.as_deref()
        .or(manifest.options.skills_dir.as_deref())
        .unwrap_or(".agents");

    // 3. Find current cached skill content
    let agents_skill = ctx.project_dir.join(".agents").join("skills").join(name);
    if !agents_skill.exists() {
        anyhow::bail!(
            "Skill '{}' is not installed. Run `ion add` first.", name
        );
    }

    // Resolve the actual content directory (follow symlinks)
    let real_source = std::fs::canonicalize(&agents_skill)?;

    // 4. Determine destination
    let dest = ctx.project_dir.join(skills_dir).join("skills").join(name);

    if dest.exists() {
        anyhow::bail!(
            "Destination already exists: {}. Remove it first.",
            dest.display()
        );
    }

    // 5. Copy skill content
    std::fs::create_dir_all(dest.parent().unwrap())?;
    copy_dir_recursive(&real_source, &dest)?;

    println!("  Copied skill content to {}", p.info(&dest.strip_prefix(&ctx.project_dir).unwrap_or(&dest).display().to_string()));

    // 6. Update symlinks
    //    If skills-dir == ".agents", the dest IS the agents location — just remove the old symlink
    //    If skills-dir != ".agents", replace .agents symlink to point at new location
    if skills_dir == ".agents" {
        // The skill was a symlink at .agents/skills/<name> → cache
        // We already created a real directory there via copy_dir_recursive above
        // But wait — we need to remove the symlink first, then the copy
        // Actually: dest == agents_skill in this case, and we checked dest doesn't exist
        // But agents_skill DOES exist (as a symlink). We need to remove it first.
        // Let's restructure: remove symlink first, then copy.

        // Actually we should handle this differently. Let me re-think.
        // The agents_skill is a symlink. We verified dest doesn't exist.
        // But if skills_dir == ".agents", then dest == agents_skill path.
        // So dest.exists() would be true if the symlink target exists. We need to check is_symlink.
    }

    // Better approach: always remove old symlink first, then copy, then re-link
    // Remove old .agents symlink
    if agents_skill.is_symlink() {
        std::fs::remove_file(&agents_skill)?;
    } else if agents_skill.is_dir() {
        // It's a real directory (e.g., path-type install). Remove it.
        std::fs::remove_dir_all(&agents_skill)?;
    }

    if skills_dir != ".agents" {
        // Create .agents/skills/<name> symlink → skills-dir location
        let rel_target = pathdiff::diff_paths(&dest, agents_skill.parent().unwrap())
            .ok_or_else(|| anyhow::anyhow!("Failed to compute relative path"))?;
        std::os::unix::fs::symlink(&rel_target, &agents_skill)?;
    } else {
        // skills-dir IS .agents, so the copy IS the agents location
        // We already removed the symlink and need to copy here
        if !dest.exists() {
            copy_dir_recursive(&real_source, &dest)?;
        }
    }

    // Target symlinks (.claude/skills/<name> etc.) already point at .agents/skills/<name>
    // so they don't need updating.

    // 7. Update Ion.toml: change to local type with forked-from
    let forked_from = match &source.source_type {
        SourceType::Github => {
            if let Some(ref path) = source.path {
                format!("{}/{}", source.source, path)
            } else {
                source.source.clone()
            }
        }
        _ => source.source.clone(),
    };

    let local_source = ion_skill::source::SkillSource {
        source_type: SourceType::Local,
        source: String::new(),
        path: None,
        rev: None,
        version: None,
        binary: None,
        asset_pattern: None,
        forked_from: Some(forked_from),
    };
    manifest_writer::remove_skill(&ctx.manifest_path, name)?;
    manifest_writer::add_skill(&ctx.manifest_path, name, &local_source)?;
    println!("  Updated {} — type changed to local", p.dim("Ion.toml"));

    // 8. Remove gitignore entries (local skills are tracked by git)
    ion_skill::gitignore::remove_skill_entries(&ctx.project_dir, name)?;
    println!("  Updated {} — removed skill entries", p.dim(".gitignore"));

    // 9. Update lockfile: drop commit, keep checksum
    let mut lockfile = ctx.lockfile()?;
    if let Some(locked) = lockfile.find(name).cloned() {
        let updated = ion_skill::lockfile::LockedSkill {
            commit: None,
            ..locked
        };
        lockfile.upsert(updated);
        lockfile.write_to(&ctx.lockfile_path)?;
        println!("  Updated {}", p.dim("Ion.lock"));
    }

    println!("{} Ejected '{}' to {}", p.success("Done!"), name,
        dest.strip_prefix(&ctx.project_dir).unwrap_or(&dest).display());
    println!("  You can now edit the skill directly. Changes are tracked by git.");

    Ok(())
}

fn copy_dir_recursive(src: &std::path::Path, dst: &std::path::Path) -> anyhow::Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        if src_path.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            std::fs::copy(&src_path, &dst_path)?;
        }
    }
    Ok(())
}
```

**Step 4: Run tests**

Run: `cargo test`
Expected: All pass (compiles correctly with new command).

**Step 5: Commit**

```bash
git add src/commands/eject.rs src/commands/mod.rs src/main.rs
git commit -m "feat: implement ion skill eject command"
```

---

### Task 8: Update `add.rs` to handle Local type in gitignore skip

**Files:**
- Modify: `src/commands/add.rs`

**Step 1: Add Local to gitignore skip conditions**

In `finish_single_install()` and `finish_collection_skill_install()`, update the condition:

```rust
if !matches!(source.source_type, SourceType::Path | SourceType::Local) {
```

**Step 2: Run tests**

Run: `cargo test`
Expected: All pass.

**Step 3: Commit**

```bash
git add src/commands/add.rs
git commit -m "feat: skip gitignore for local skills in add command"
```

---

### Task 9: Update `installer.rs` to handle Local in `build_locked_entry`

**Files:**
- Modify: `crates/ion-skill/src/installer.rs`

**Step 1: Handle Local type alongside Path**

In `build_locked_entry()`, add `Local` to the Path/Http/Binary arm:

```rust
SourceType::Path | SourceType::Http | SourceType::Binary | SourceType::Local => {
    let checksum = git::checksum_dir(skill_dir).ok();
    (None, checksum)
}
```

Also add `forked_from: None` to all `SkillSource` construction in `fetch_skill_base` and anywhere else that constructs `SkillSource`.

**Step 2: Run tests**

Run: `cargo test`
Expected: All pass.

**Step 3: Commit**

```bash
git add crates/ion-skill/src/installer.rs
git commit -m "feat: handle Local source type in installer"
```

---

### Task 10: Integration tests

**Files:**
- Modify: `tests/integration.rs`

**Step 1: Test local skill in Ion.toml install flow**

```rust
#[test]
fn install_local_skill_ensures_symlinks() {
    let project = tempfile::tempdir().unwrap();

    // Create a local skill under .agents/skills/
    let skill_dir = project.path().join(".agents/skills/my-local");
    std::fs::create_dir_all(&skill_dir).unwrap();
    std::fs::write(
        skill_dir.join("SKILL.md"),
        "---\nname: my-local\ndescription: A local skill.\n---\n\nLocal body.\n",
    ).unwrap();

    // Create Ion.toml with local skill and a target
    std::fs::write(
        project.path().join("Ion.toml"),
        "[skills]\nmy-local = { type = \"local\" }\n\n[options.targets]\nclaude = \".claude/skills\"\n",
    ).unwrap();

    let output = ion_cmd()
        .args(["add"])
        .current_dir(project.path())
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "install local failed: stdout={stdout}\nstderr={stderr}");
    assert!(stdout.contains("local"));

    // Target symlink should exist
    let target = project.path().join(".claude/skills/my-local");
    assert!(target.exists(), "target symlink should exist");

    // No gitignore entry for local skills
    let gitignore = project.path().join(".gitignore");
    if gitignore.exists() {
        let content = std::fs::read_to_string(&gitignore).unwrap();
        assert!(!content.contains("my-local"), "local skills should not be gitignored");
    }
}
```

**Step 2: Test remove preserves local skill directory**

```rust
#[test]
fn remove_local_skill_preserves_directory() {
    let project = tempfile::tempdir().unwrap();

    // Create a local skill
    let skill_dir = project.path().join(".agents/skills/my-local");
    std::fs::create_dir_all(&skill_dir).unwrap();
    std::fs::write(
        skill_dir.join("SKILL.md"),
        "---\nname: my-local\ndescription: A local skill.\n---\n\nBody.\n",
    ).unwrap();

    std::fs::write(
        project.path().join("Ion.toml"),
        "[skills]\nmy-local = { type = \"local\" }\n",
    ).unwrap();
    std::fs::write(
        project.path().join("Ion.lock"),
        "version = 1\n\n[skills]\n",
    ).unwrap();

    let output = ion_cmd()
        .args(["remove", "my-local", "-y"])
        .current_dir(project.path())
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "remove failed: stdout={stdout}\nstderr={stderr}");

    // Skill directory should still exist (preserved)
    assert!(skill_dir.exists(), "local skill directory should be preserved");

    // But Ion.toml should no longer have the entry
    let manifest = std::fs::read_to_string(project.path().join("Ion.toml")).unwrap();
    assert!(!manifest.contains("my-local"));
}
```

**Step 3: Run tests**

Run: `cargo test`
Expected: All pass including new integration tests.

**Step 4: Commit**

```bash
git add tests/integration.rs
git commit -m "test: add integration tests for local skills"
```

---

### Task 11: Integration test for `ion skill eject`

**Files:**
- Modify: `tests/integration.rs`

**Step 1: Test eject command**

```rust
#[test]
fn eject_converts_remote_to_local() {
    let project = tempfile::tempdir().unwrap();
    let skill_base = tempfile::tempdir().unwrap();
    let skill_path = skill_base.path().join("eject-test");
    std::fs::create_dir(&skill_path).unwrap();
    std::fs::write(
        skill_path.join("SKILL.md"),
        "---\nname: eject-test\ndescription: Eject test.\n---\n\nBody.\n",
    ).unwrap();

    // First, add the skill as a path skill
    let output = ion_cmd()
        .args(["add", &skill_path.display().to_string()])
        .current_dir(project.path())
        .output()
        .unwrap();
    assert!(output.status.success(), "add failed");

    // Eject it
    let output = ion_cmd()
        .args(["skill", "eject", "eject-test"])
        .current_dir(project.path())
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "eject failed: stdout={stdout}\nstderr={stderr}");

    // Ion.toml should now have local type
    let manifest = std::fs::read_to_string(project.path().join("Ion.toml")).unwrap();
    assert!(manifest.contains("local"), "should be local type: {manifest}");

    // Skill content should exist as real directory
    let ejected = project.path().join(".agents/skills/eject-test");
    assert!(ejected.exists());
    assert!(ejected.join("SKILL.md").exists());
    assert!(!ejected.is_symlink(), "should be real directory, not symlink");
}
```

**Step 2: Test eject errors for already-local skill**

```rust
#[test]
fn eject_errors_for_local_skill() {
    let project = tempfile::tempdir().unwrap();

    std::fs::write(
        project.path().join("Ion.toml"),
        "[skills]\nmy-local = { type = \"local\" }\n",
    ).unwrap();

    let output = ion_cmd()
        .args(["skill", "eject", "my-local"])
        .current_dir(project.path())
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("already local"));
}
```

**Step 3: Run tests**

Run: `cargo test`
Expected: All pass.

**Step 4: Commit**

```bash
git add tests/integration.rs
git commit -m "test: add integration tests for ion skill eject"
```

---

### Task 12: Update `--help` test and verify end-to-end

**Files:**
- Modify: `tests/integration.rs`

**Step 1: Update help test to include eject**

Add `eject` to the `help_shows_all_commands` test's skill subcommand check, or add a dedicated help test:

```rust
#[test]
fn skill_eject_help_is_exposed() {
    let output = ion_cmd().args(["skill", "eject", "--help"]).output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success());
    assert!(stdout.contains("Eject") || stdout.contains("eject"));
}
```

**Step 2: Run full test suite**

Run: `cargo test`
Expected: All tests pass.

Run: `cargo clippy`
Expected: No warnings.

**Step 3: Commit**

```bash
git add tests/integration.rs
git commit -m "test: add eject help test and verify full suite"
```
