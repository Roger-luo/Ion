# Scenario Crate: Project Fixture System

**Date:** 2026-03-28
**Status:** Design approved

## Problem

The scenario crate handles *how* CLI commands run (terminal modes, timeouts, interactive sessions) but says nothing about *where* they run. Project setup is entirely ad-hoc — each test file has its own helper functions that manually create tempdirs, write config files, and create directory structures. This leads to duplication, inconsistency, and makes it hard to see what project states are being tested.

## Goals

- Formally manage project structures/setups that CLI tests run inside
- Generic — not Ion-specific; usable by any CLI project
- Template-based: project structures defined as inspectable on-disk templates, parameterized via Rust API
- Support the full range of project artifacts: files, directories, and symlinks

## Design

### Core Types and Builder API

`Project` owns a tempdir (or references a caller-provided path) containing materialized project files. Created via a builder pattern with two entry points.

```rust
pub struct Project {
    dir: ProjectDir,  // enum { Temp(TempDir), External(PathBuf) }
}

impl Project {
    /// Path to the materialized project directory.
    pub fn path(&self) -> &Path;

    /// Start from a template directory on disk.
    pub fn from_template(path: impl AsRef<Path>) -> ProjectBuilder;

    /// Start from an empty directory (escape hatch for one-offs).
    pub fn empty() -> ProjectBuilder;
}
```

Builder methods (all optional, chainable):

```rust
impl ProjectBuilder {
    /// Set a template variable.
    fn var(self, key: &str, value: &str) -> Self;

    /// Set multiple template variables.
    fn vars(self, vars: impl IntoIterator<Item = (&str, &str)>) -> Self;

    /// Exclude a file or directory from the template output (exact path or prefix for dirs).
    fn exclude(self, path: &str) -> Self;

    /// Re-include a file/dir that is optional by default (exact path or prefix for dirs).
    fn include(self, path: &str) -> Self;

    /// Replace a template file with custom content (still rendered through minijinja).
    fn override_file(self, path: &str, content: &str) -> Self;

    /// Add a file not in the template (written verbatim, no rendering).
    fn file(self, path: &str, content: &str) -> Self;

    /// Create an empty directory.
    fn dir(self, path: &str) -> Self;

    /// Build into a new tempdir. Cleaned up on drop.
    fn build(self) -> Result<Project>;

    /// Build into an existing directory. Caller manages lifecycle.
    fn build_in(self, path: impl AsRef<Path>) -> Result<Project>;
}
```

### Integration with Scenario

`Project` integrates as an optional method in the `Scenario` builder chain. It is sugar for `.current_dir(project.path())`.

```rust
impl Scenario {
    pub fn project(self, project: &Project) -> Self {
        self.current_dir(project.path())
    }
}
```

The caller holds both `Project` and `Scenario` — no ownership transfer. The project must outlive the scenario execution.

```rust
let project = Project::from_template("tests/fixtures/basic-project")
    .var("name", "my-skill")
    .build()?;

let output = Scenario::new(ION)
    .project(&project)
    .args(["skill", "list"])
    .run()?;
```

### Template Directory Structure

A template is a plain directory on disk. All files in it are rendered through minijinja, except for `template.toml` which is the manifest.

Example template:

```
tests/fixtures/basic-project/
├── template.toml
├── Ion.toml
└── skills/
    └── SKILL.md
```

Files with templated destination paths (e.g., `.agents/skills/{{name}}/SKILL.md`) are not stored under templated directory names on disk. Instead, path mappings in the manifest route clean source paths to rendered destinations.

### `template.toml` Manifest

```toml
[variables]
name = { description = "Skill name", default = "test-skill" }
description = { description = "Skill description" }  # required, no default
version = { default = "1.0" }                         # description optional

[files]
# Glob patterns for files excluded by default (opt-in via .include())
optional = [
    "Ion.lock",
    ".claude/",
]

[files.mappings]
# source (in template dir) = destination (in output, rendered through minijinja)
"skills/SKILL.md" = ".agents/skills/{{name}}/SKILL.md"

[files.symlinks]
# link path (rendered) = link target (rendered)
".claude/skills/{{name}}" = "../../.agents/skills/{{name}}"
```

**Sections:**

- **`[variables]`** — Declares available template variables. Each entry is a table with optional `description` and `default` fields. Variables without a `default` are required and must be provided via `.var()` or `.vars()`.
- **`[files.mappings]`** — Maps source file paths (in the template directory) to destination paths (in the output). Both source and destination are rendered through minijinja. Files not listed keep their natural path relative to the template root.
- **`[files.symlinks]`** — Declares symlinks to create. Both link path and target are rendered through minijinja. Symlinks are created after all files are written so targets exist.
- **`[files].optional`** — Glob patterns for files/directories excluded by default. Callers opt them in via `.include()`.

### Template Rendering Pipeline

When `.build()` or `.build_in()` is called:

**Step 1: Load manifest.** Parse `template.toml` from the template directory. For `Project::empty()`, skip to step 5.

**Step 2: Validate variables.** Check that all required variables (no `default`) have been provided. Fill in defaults for unprovided optional variables. Error if unknown variables were set (catches typos). Report *all* missing variables, not just the first.

**Step 3: Compute file set.** Walk the template directory, excluding `template.toml` itself:
1. Start with all files
2. Remove files matching `[files].optional` patterns
3. Add back anything the caller `.include()`'d
4. Remove anything the caller `.exclude()`'d
5. Layer on `.override_file()` and `.file()` additions

**Step 4: Render and write.** For each file in the computed set:
1. Determine destination path — use `[files.mappings]` if listed, otherwise the file's natural path
2. Render the destination path through minijinja
3. Render the file content through minijinja
4. Write to the target directory, creating parent dirs as needed

For `.override_file()` entries: content is also rendered through minijinja, so overrides can reference template variables.

**Step 5: Apply programmatic additions.** Write any `.file()` and `.dir()` entries that aren't overrides. These are written verbatim (no template rendering).

**Step 6: Create symlinks.** Process `[files.symlinks]` entries. Render both path and target through minijinja. Create symlinks in the target directory.

### Error Handling

New error variants added to `scenario::Error`:

| Variant | When |
|---------|------|
| `TemplateNotFound { path }` | Template directory doesn't exist |
| `ManifestParse { path, source }` | `template.toml` is invalid TOML or has bad structure |
| `MissingVariable { names: Vec<String> }` | Required variables not provided (lists all) |
| `UnknownVariable { name }` | `.var()` called with undeclared variable name |
| `TemplateRender { file, source }` | Minijinja render error, includes file path and line/column |
| `SymlinkTarget { path }` | Symlink target doesn't exist after file write |

### Dependencies

- **minijinja** — lightweight Jinja2-compatible template engine (~35k lines, no transitive deps). Used for variable substitution, conditionals, loops in template files and paths.

### Testing Strategy

Tests live in `crates/scenario/tests/` alongside the existing `integration.rs`. A small set of template directories under `crates/scenario/tests/fixtures/` provide test data.

**Coverage:**

- Basic rendering — `from_template` with variables, verify files land with correct content and paths
- Empty project — `Project::empty().file(...).build()` creates the right structure
- Variable validation — missing required var errors, unknown var errors, defaults applied
- File filtering — `optional` excluded by default, `.include()` brings them back, `.exclude()` removes non-optional files
- Override and additions — `.override_file()` replaces template content (still rendered), `.file()` adds verbatim
- Path mappings — `[files.mappings]` routes source files to rendered destinations
- Symlinks — `[files.symlinks]` creates working symlinks with rendered paths
- `build_in()` — writes to caller-specified path instead of tempdir
- Cleanup — `Project` from `.build()` cleans up tempdir on drop
- Template errors — bad minijinja syntax, missing template dir, malformed `template.toml`

## Usage Example (Ion-specific, in ion test code)

```rust
const ION: &str = env!("CARGO_BIN_EXE_ion");

fn ion() -> Scenario {
    Scenario::new(ION).timeout(Duration::from_secs(10))
}

#[test]
fn skill_list_shows_local_skill() {
    let project = Project::from_template("tests/fixtures/ion-with-skill")
        .var("name", "my-skill")
        .var("description", "A test skill")
        .build()
        .unwrap();

    let output = ion()
        .project(&project)
        .args(["skill", "list"])
        .run()
        .unwrap();

    assert!(output.success());
    assert!(output.stdout().contains("my-skill"));
}

#[test]
fn init_refuses_existing_manifest() {
    let project = Project::from_template("tests/fixtures/ion-with-skill")
        .var("name", "test")
        .build()
        .unwrap();

    let output = ion()
        .project(&project)
        .args(["init"])
        .run()
        .unwrap();

    assert!(!output.success());
    assert!(output.stderr().contains("already exists"));
}

#[test]
fn init_works_in_empty_dir() {
    let project = Project::empty().build().unwrap();

    let output = ion()
        .project(&project)
        .args(["init"])
        .run()
        .unwrap();

    assert!(output.success());
}
```
