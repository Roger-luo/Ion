---
title: "ion-skill::error"
description: "Error types for the ion-skill library, covering IO, parsing, Git, HTTP, validation, and manifest failures."
order: 999
---

Error types for the ion-skill library, covering IO, parsing, Git, HTTP, validation, and manifest failures.

## Error

### Variants

- **`Io(io::Error)`**
- **`TomlParse(Error)`**
- **`TomlEdit(TomlError)`**
- **`YamlParse(Error)`**
- **`InvalidSkill(String)`**
- **`Source(String)`**
- **`Git(String)`**
- **`Manifest(String)`**
- **`Search(String)`**
- **`Http(String)`**
- **`Other(String)`**
- **`ValidationFailed { report: ValidationReport, error_count: usize, warning_count: usize, info_count: usize }`**
- **`ValidationWarning { report: ValidationReport, warning_count: usize, info_count: usize }`**

### Methods

#### `validation_failed`

```rust
pub fn validation_failed(report: ValidationReport) -> Self
```

Create a ValidationFailed error from a report.

#### `validation_warning`

```rust
pub fn validation_warning(report: ValidationReport) -> Self
```

Create a ValidationWarning error from a report.

### Trait Implementations

- `Debug`
- `Error`
- `Display`
- `From<Error>`
- `From<Error>`
- `From<TomlError>`
- `From<Error>`
- `From<CliError>`

