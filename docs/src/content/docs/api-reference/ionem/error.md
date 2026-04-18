---
title: "ionem::error"
description: "Error types for the ionem library — IO, HTTP, and general errors from binary skill operations."
order: 999
---

## Error

### Variants

- **`Io(io::Error)`**
- **`Http(String)`**
- **`Other(String)`**

### Trait Implementations

- `Debug`
- `Error`
- `Display`
- `From<Error>`

