# Ion - a CLI toolbox for Julia developer

[![Build](https://github.com/Roger-luo/Ion/actions/workflows/main.yml/badge.svg)](https://github.com/Roger-luo/Ion/actions/workflows/main.yml)
[![codecov](https://codecov.io/gh/Roger-luo/ion/branch/main/graph/badge.svg?token=3PIJaVaOkT)](https://codecov.io/gh/Roger-luo/ion)

Ion is a CLI toolbox for Julia developer. It provides a set of tools to help you develop Julia packages.

Announcement: https://discourse.julialang.org/t/ann-the-ion-command-line-for-julia-developers-written-in-rust/94495

## Installation

### pre-build binary

Download tarball in the release page and extract it to your `$HOME/.julia` directory.

### build from source

Using [`just`](https://github.com/casey/just) and [Rust's cargo/rustc compiler](https://rustup.rs/):

```bash
just install
```

## License

MIT License
