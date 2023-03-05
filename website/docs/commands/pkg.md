---
sidebar_position: 1
---

# Julia Pkg Commands

The Julia package manager is called `Pkg`. It is used to install, update, and remove packages, as well as to manage the versions of packages in your project. The `Pkg` module is part of the standard library and is always available.

Ion forwards Julia's Pkg commands in the terminal, e.g

```sh
> ion add -h
Add dependencies to current environment

Usage: ion add [OPTIONS] [PACKAGE]...

Arguments:
  [PACKAGE]...  The package to add

Options:
  -g, --global  Add the package to the global environment
  -h, --help    Print help
```

and all Pkg commands will run as equivalent to having `julia --project -e "using Pkg; ...` by default, and there is a `-g --global` option to manage global shared environment.
