---
title: "ion-skill::git"
description: "Git operations for skill management — clone, fetch, checkout, and compute directory checksums."
order: 999
---

## clone_or_fetch

```rust
pub fn clone_or_fetch(url: &str, target: &Path) -> Result<()>
```

Clone a git repository to a target directory. If it already exists, fetch updates.

---

## checkout

```rust
pub fn checkout(repo_path: &Path, rev: &str) -> Result<()>
```

Checkout a specific ref (branch, tag, or commit SHA).

---

## head_commit

```rust
pub fn head_commit(repo_path: &Path) -> Result<String>
```

Get the current HEAD commit SHA.

---

## checksum_dir

```rust
pub fn checksum_dir(dir: &Path) -> Result<String>
```

Compute a SHA-256 checksum of a directory's contents (all files, sorted).

---

## default_branch

```rust
pub fn default_branch(repo_path: &Path) -> Result<String>
```

Get the default branch name for a repo by checking `origin/HEAD` or falling back
to `symbolic-ref HEAD`.

---

## reset_to_remote_head

```rust
pub fn reset_to_remote_head(repo_path: &Path) -> Result<()>
```

Reset the working tree to the remote's default branch HEAD.
Call this after `clone_or_fetch()` to advance to the latest commit.

