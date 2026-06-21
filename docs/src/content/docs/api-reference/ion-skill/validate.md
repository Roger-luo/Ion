---
title: "ion-skill::validate"
description: "Skill validation framework — run checkers against SKILL.md files and aggregate findings by severity."
order: 999
---

Skill validation framework — run checkers against SKILL.md files and aggregate findings by severity.

## Finding

A single validation finding produced by a checker.

### Fields

| Name | Type | Description |
|------|------|-------------|
| `severity` | `Severity` |  |
| `checker` | `String` |  |
| `message` | `String` |  |
| `detail` | `Option<String>` |  |

### Trait Implementations

- `Debug`
- `Clone`
- `Serialize`

---

## ValidationReport

Aggregated validation output for a single skill.

### Fields

| Name | Type | Description |
|------|------|-------------|
| `findings` | `Vec<Finding>` |  |
| `error_count` | `usize` |  |
| `warning_count` | `usize` |  |
| `info_count` | `usize` |  |

### Methods

#### `from_findings`

```rust
pub fn from_findings(findings: Vec<Finding>) -> Self
```

### Trait Implementations

- `Debug`
- `Clone`
- `Serialize`

---

## Severity

How severe a validation finding is.

### Variants

- **`Info`**
- **`Warning`**
- **`Error`**

### Trait Implementations

- `Debug`
- `Clone`
- `PartialEq`
- `Eq`
- `Hash`
- `Serialize`
- `Display`
- `Ord`
- `PartialOrd`

---

## run_all_checkers

```rust
pub fn run_all_checkers(skill_dir: &Path, meta: &SkillMetadata, body: &str) -> Vec<Finding>
```

Run every registered checker and return all findings sorted by severity
descending (errors first).

---

## validate_skill_dir

```rust
pub fn validate_skill_dir(skill_dir: &Path, meta: &SkillMetadata, body: &str) -> ValidationReport
```

Run validation and return an aggregated report with counts.

---

## has_errors

```rust
pub fn has_errors(findings: &[Finding]) -> bool
```

Returns `true` if any finding has `Severity::Error`.

---

## has_warnings

```rust
pub fn has_warnings(findings: &[Finding]) -> bool
```

Returns `true` if any finding has `Severity::Warning`.

