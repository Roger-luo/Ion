---
sidebar_position: 2
---

# Installation

The installation process is currently a bit manual, but we are working on making
it easier.

You can download the pre-built binary from the [releases page](https://github.com/Roger-luo/Ion/releases), there is only one binary inside each tarball, you can put it anywhere you want, and add it to your `PATH`. Usually you can just put it in `~/.local/bin` and add `~/.local/bin` to your `PATH`.

## Install from source

Ion is written in rust, so you need to have [rust installed](https://www.rust-lang.org/learn/get-started) to build it from source.

```bash
git clone https://github.com/Roger-luo/Ion.git
cd Ion
cargo install --path .
```

Or you can use [`just`](https://github.com/casey/just)

```bash
just install
```

## Optional prerequisites

- `git`: some Ion commands requires git to be installed, such as using `ion new`
  to setup a new project with git repo.
- `julia`: Ion forwards Julia pkg commands to manage current `julia` environment.
  If you don't have `julia` installed, Ion will still work, but you won't be able
  to use Julia pkg commands.

## Shell auto-completion

If you want to have shell auto-completion, after downloading ion, run `ion completions <shell>` to generate the shell completion script, e.g for `oh-my-zsh` you can copy and paste the following

```sh
cd .oh-my-zsh/completions
ion completions zsh |> _ion
```

In the future, if this is adopted by more people, maybe we can have juliaup ship this or have an ionup for a friendlier installation process.
