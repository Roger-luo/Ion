# Design: Extend `ion update` to Non-Binary Skills

## Summary

Extend `ion update` to support git/GitHub skills in addition to binary skills, using an `Updater` trait with source-type-specific implementations.

## Requirements

- Git/GitHub skills without a pinned `rev` pull latest from the default branch
- Skills with a `rev` set in Ion.toml are silently skipped (pinned)
- Path skills are silently skipped (user-managed)
- HTTP skills are skipped (not yet implemented)
- Full validation runs on new versions before accepting the update
- Validation failure leaves the old version intact

## Architecture

New module `ion-skill/src/update/` with:

```
update/
├── mod.rs          # Updater trait, UpdateInfo, UpdateContext, dispatch
├── binary.rs       # BinaryUpdater (extracted from current update.rs)
└── git.rs          # GitUpdater (for Git + GitHub source types)
```

### Updater Trait

```rust
pub trait Updater {
    fn check(&self, skill: &LockedSkill, source: &SkillSource) -> Result<Option<UpdateInfo>>;
    fn apply(&self, skill: &LockedSkill, source: &SkillSource, ctx: &UpdateContext) -> Result<LockedSkill>;
}
```

### Git/GitHub Update Flow

1. **Check:** `clone_or_fetch()` on cached repo, compare HEAD commit against lockfile commit
2. **Apply:** Checkout new HEAD, resolve subdirectory path, validate, compute checksum, re-deploy symlinks, return updated `LockedSkill`

### Binary Update Flow

Same as current logic, extracted into `BinaryUpdater` implementing the trait.

### CLI Orchestration (`update.rs`)

- Iterate manifest skills, match source type to updater
- Skip pinned (has `rev`), path, and HTTP skills
- Call `check()` then `apply()` for each
- Write lockfile once at the end

## Error Handling

- Network errors or deleted repos: warn, skip, continue
- Validation failure: warn with findings, keep old version, continue
- Individual failures don't abort the update run
- Lockfile only written if at least one skill updated

## Output Format

```
Updating skills...
  ✓ my-skill         abc1234 → def5678
  ✓ another-tool     v1.0.0 → v1.1.0  (binary)
  - pinned-skill     skipped (pinned to v2.0.0)
  ✗ broken-skill     validation failed: security issue found
  · unchanged-skill  already up to date

Updated 2 skills, 1 skipped, 1 failed, 1 up to date
```

## Testing

- Unit tests for `GitUpdater` and `BinaryUpdater` with mocked repos
- Integration test: temp git repo skill → install → upstream commit → `ion update` → verify lockfile
- Integration test: pinned rev is skipped
- Integration test: validation failure preserves old version
