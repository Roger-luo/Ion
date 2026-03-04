# Binary CLI Skills Phase 2: Lifecycle Management

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add lifecycle management for binary skills — update checking, smart caching during install, and cleanup on remove.

**Architecture:** Extend `binary.rs` with cache-check and cleanup functions. Add `ion update` command. Modify installer to skip downloads when binary is cached. Extend remove to clean up binary files.

**Tech Stack:** Same as Phase 1 — Rust, reqwest, flate2/tar, clap, serde/toml

---

## Task 1: Add binary cache check to skip redundant downloads

**Files:**
- Modify: `crates/ion-skill/src/binary.rs`
- Modify: `crates/ion-skill/src/installer.rs`

Add a function to check if a binary is already installed at the expected version:

```rust
/// Check if a binary is already installed at the given version.
pub fn is_binary_installed(name: &str, version: &str) -> bool {
    binary_path(name, version).exists()
}
```

Then modify `install_binary` in installer.rs to check the lockfile first — if the binary is already installed at the locked version, skip the download and just ensure the SKILL.md and symlinks are in place.

**Tests:**
- Unit test for `is_binary_installed` (false when missing, true after install)
- Verify installer skips download when binary exists

---

## Task 2: Add binary cleanup functions

**Files:**
- Modify: `crates/ion-skill/src/binary.rs`

Add functions to remove binaries from storage:

```rust
/// Remove a specific version of a binary.
pub fn remove_binary_version(name: &str, version: &str) -> crate::Result<()>

/// Remove all versions of a binary (the entire {bin_dir}/{name}/ directory).
pub fn remove_binary(name: &str) -> crate::Result<()>

/// List all installed binary names.
pub fn list_installed_binaries() -> crate::Result<Vec<String>>
```

**Tests:**
- Install then remove, verify directory is gone
- Remove non-existent binary is a no-op (not an error)

---

## Task 3: Wire binary cleanup into ion remove

**Files:**
- Modify: `src/commands/remove.rs`
- Modify: `crates/ion-skill/src/installer.rs`

After removing symlinks, check if the removed skill was a binary skill (check lockfile entry for `binary` field). If so, call `binary::remove_binary()` to clean up the binary files.

The remove command already has the lockfile loaded. Check `locked.binary` before removing.

---

## Task 4: Implement ion update command

**Files:**
- Create: `src/commands/update.rs`
- Modify: `src/commands/mod.rs`
- Modify: `src/main.rs`

The update command:
1. If a skill name is given, update just that skill
2. If no name, update all binary skills
3. For each binary skill:
   a. Read current version from lockfile
   b. Query GitHub for latest release
   c. Compare versions — skip if up-to-date
   d. Download new version, install alongside old
   e. Re-run `<binary> skill` to regenerate SKILL.md
   f. Update lockfile with new version/checksum
   g. Print what was updated

CLI: `ion update [name]`

---

## Task 5: Integration tests for Phase 2

**Files:**
- Modify: `tests/binary_integration.rs`

Add tests for:
- Binary cache check (is_binary_installed)
- Binary cleanup (remove_binary)
- Mixed manifest with binary and regular skills through install flow
