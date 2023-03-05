---
sidebar_position: 2
---


# Create a new package with pre-defined templates

- Have you ever typed the same project configuration again and again interactively with `PkgTemplates`?
- Have you ever typed the wrong option in the interactive mode with `PkgTemplates` and had to start over entirely?

The `ion new` command is here to help: we create an entirely new templating system based on `PkgTemplates` but with serialization in TOML.

The following is a project template for a small project:

```toml
name="project"
description = "A project description"

[readme]
[project_file]
[src_dir]
[tests]
```

and the following is a project template for research packages:

```toml
name="research"
description = "A research package description"

[project_file]
[readme]
[src_dir]
[documenter]
[license]
[tests]
[repo]
[codecov]
[citation]
[github.ci]
arch = ["x86", "x86_64"]
os = ["ubuntu-latest", "macos-latest", "windows-latest"]

[github.tagbot]
[github.compat_helper]
```

Most importantly you can save your own custom configuration and share it with people! Maybe your company's internal packages need a custom `README` template and `LICENSE`? Create your own `template.toml` with corresponding components and share it with your colleagues instead of asking them to do it interactively! Check examples in our [template registry here](https://github.com/Roger-luo/ion-templates)!
