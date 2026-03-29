---
title: "ionem::shell::git"
description: "Git CLI wrappers."
order: 999
---

Git CLI wrappers.

Use [`require()`] to verify `git` is installed, then call methods on the
returned [`Git`] handle:

```ignore
let git = git::require()?;
let repo = git.repo(project_dir);
repo.stage_files(&["Ion.toml", "Ion.lock"])?;
if repo.has_staged_changes()? {
    let sha = repo.create_commit("chore: update manifest")?;
}
```

## Git

A validated handle proving the `git` CLI is available.

Obtained via [`require()`]. Context constructors and standalone
operations live here.

### Methods

#### `repo`

```rust
pub fn repo<'a>(&self, path: &'a Path) -> Repo<'a>
```

Create a [`Repo`] context bound to the given working directory.

#### `clone_or_fetch`

```rust
pub fn clone_or_fetch(&self, url: &str, target: &Path) -> Result<()>
```

Clone a git repository, or fetch updates if it already exists.

#### `init`

```rust
pub fn init(&self, path: &Path) -> Result<()>
```

Initialize a new git repository.

---

## Repo

A git repository context that binds a working directory, so you can call
multiple operations without repeating the path.

*…and private fields*

### Methods

#### `checkout`

```rust
pub fn checkout(&self, rev: &str) -> Result<()>
```

Checkout a specific ref (branch, tag, or commit SHA).

#### `head_commit`

```rust
pub fn head_commit(&self) -> Result<String>
```

Get the current HEAD commit SHA.

#### `default_branch`

```rust
pub fn default_branch(&self) -> Result<String>
```

Get the default branch name.

#### `reset_to_remote_head`

```rust
pub fn reset_to_remote_head(&self) -> Result<()>
```

Reset the working tree to the remote's default branch HEAD.

#### `stage_files`

```rust
pub fn stage_files(&self, files: &[&str]) -> Result<()>
```

Stage files.

#### `has_staged_changes`

```rust
pub fn has_staged_changes(&self) -> Result<bool>
```

Check if there are staged changes.

#### `create_commit`

```rust
pub fn create_commit(&self, message: &str) -> Result<String>
```

Create a commit with the given message and return the new HEAD SHA.

#### `fetch_all`

```rust
pub fn fetch_all(&self) -> Result<()>
```

Fetch updates from all remotes (requires an existing clone).

---

## require

```rust
pub fn require() -> Result<Git>
```

Verify `git` is installed and return a handle to run commands.

---

## clone_or_fetch

```rust
pub fn clone_or_fetch(url: &str, target: &Path) -> Result<()>
```

Clone a git repository to a target directory. If it already exists, fetch updates.

---

## checkout

```rust
pub fn checkout(repo: &Path, rev: &str) -> Result<()>
```

Checkout a specific ref (branch, tag, or commit SHA).

---

## head_commit

```rust
pub fn head_commit(repo: &Path) -> Result<String>
```

Get the current HEAD commit SHA.

---

## default_branch

```rust
pub fn default_branch(repo: &Path) -> Result<String>
```

Get the default branch name for a repo by checking `origin/HEAD` or falling back
to `symbolic-ref HEAD`.

---

## reset_to_remote_head

```rust
pub fn reset_to_remote_head(repo: &Path) -> Result<()>
```

Reset the working tree to the remote's default branch HEAD.
Call this after `clone_or_fetch()` to advance to the latest commit.

---

## stage_files

```rust
pub fn stage_files(repo: &Path, files: &[&str]) -> Result<()>
```

Stage files in a git repository.

---

## has_staged_changes

```rust
pub fn has_staged_changes(repo: &Path) -> Result<bool>
```

Check if there are staged changes in a git repository.

Returns `true` if there are staged changes, `false` if there are none.
`git diff --cached --quiet` exits with code 1 when there are changes.

---

## create_commit

```rust
pub fn create_commit(repo: &Path, message: &str) -> Result<String>
```

Create a commit with the given message and return the new HEAD commit SHA.

---

## init

```rust
pub fn init(path: &Path) -> Result<()>
```

Initialize a new git repository at the given path.

---

## repo

```rust
pub fn repo(path: &Path) -> Repo<'_>
```

Create a [`Repo`] context bound to the given working directory.

