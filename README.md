# Ion - a CLI toolbox for Julia developers

[![Build](https://github.com/Roger-luo/Ion/actions/workflows/main.yml/badge.svg)](https://github.com/Roger-luo/Ion/actions/workflows/main.yml)
[![codecov](https://codecov.io/gh/Roger-luo/ion/branch/main/graph/badge.svg?token=3PIJaVaOkT)](https://codecov.io/gh/Roger-luo/ion)
[![GitHub commits since tagged version](https://img.shields.io/github/commits-since/Roger-luo/Ion/v0.1.15.svg)](https://Roger-luo.github.io/Ion)

Ion is a CLI toolbox for Julia developer. It provides a set of tools to help you develop Julia packages.

## Installation

### pre-build binary

Download tarball in the release page and extract it to your `$HOME/.local` directory, remember to add this to your `PATH`
environment variable if you haven't done so.

### build from source

Using [`just`](https://github.com/casey/just) and [Rust's cargo/rustc compiler](https://rustup.rs/):

```bash
just install
```

## Documentation

- [Ion website](https://rogerluo.dev/Ion/)
- check `ion --help` for more information in the terminal

## License

MIT License
