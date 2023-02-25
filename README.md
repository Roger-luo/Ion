# Ion - a CLI toolbox for Julia developer

[![Build](https://github.com/Roger-luo/Ion/actions/workflows/main.yml/badge.svg)](https://github.com/Roger-luo/Ion/actions/workflows/main.yml)
[![codecov](https://codecov.io/gh/Roger-luo/ion/branch/main/graph/badge.svg?token=3PIJaVaOkT)](https://codecov.io/gh/Roger-luo/ion)
[![GitHub commits since tagged version](https://img.shields.io/github/commits-since/Roger-luo/Ion/v0.1.15.svg)](https://Roger-luo.github.io/Ion)

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


### Overview
Everything in Ion is optimized for terminal UX, including its name - ion is the easiest-to-type name that I find not used by other popular CLI tools.

This is still at quite an early stage, but I have been using it for a month myself, so I'd like to share this tool with the community. There will be more detailed documentation and a website set up in the future.

TL;DR let me introduce what commands ion provides currently, you can find more detailed information in the help message by running ion help.

#### Installation - Details

Currently only MacOS and Linux are supported; the MacOS and Linux binaries are packed as a tarball in the release CDN. There is only one binary in the release, so just download it and put it wherever you like. Also, if anyone would like to contribute their knowledge about how to release Windows binaries, that would be appreciated!

Or you can install by building it from a source if you have a rust compiler setup locally, after cloning the repo, you can just run just install and it will install the binary to the .local/bin/ folder.

If you want to have shell auto-completion, after downloading ion, run ion completions <shell> to generate the shell completion script, e.g for oh-my-zsh you can copy and paste the following

> cd .oh-my-zsh/completions

> ion completions zsh |> _ion

In the future, if this is adopted by more people, maybe we can have juliaup ship this or have an ionup for a friendlier installation process.

#### The forwarded Pkg commands

Ion forwards Julia's Pkg commands in the terminal, e.g

> ion add

Add dependencies to current environment


Usage:
```
ion add [OPTIONS] [PACKAGE]...

Arguments:
  [PACKAGE]...  The package to add

Options:
  -g, --global  Add the package to the global environment
  -h, --help    Print help
```
and all Pkg commands will run as equivalent to having `julia --project -e "using Pkg; ...` by default, and there is a `-g --global` option to manage global shared environment.

#### Releasing a new version with Ion

This has been one of the most frequently used features personally in my previous Julia version, and in this new rust version, I have rewritten the whole thing in a much cleaner and modular fashion.

Have you tired of wanting to bump a patch version, but forgetting it's already bumped in main and only noticing it's a wrong patch version after the General registry complains to you after 3min? Have you been tired of opening your editor, changing the version in Project.toml manually then opening a browser to summon JuliaRegistrator but having the bot name typed wrong?

With ion release you only need one line to do all these within seconds! e.g

> ion release patch

will automatically bump a patch version based on your current version in main and your registered version. If it's not a continuous version number, ion will warn you and ask if you want to continue; if your current version number is already valid, ion will ask if you want to release that instead - and Ion will also summon JuliaRegistrator directly without asking you to open the browser.

#### Releases
Custom release pipeline with summon and bump:

We now also provide bump and summon commands to allow you to do more customization, e.g managing large mono-repo that contains many other packages or maybe you want to pack up some artifact before summon JuliaRegistrator. You can combine these two commands with [`just`](https://github.com/casey/just) (recommended) or `make`, like what I do here.


#### Run a standalone script with dependencies

This is a feature that is supported in many other languages: Single-file scripts that download their dependencies. The lack of this feature has been a gripe from users in the Julia community; Ion solves this pain point with its new script system.

You can write the following in your Julia script
```Julia
# !/usr/bin/env ion run
#=ion
Example = "0.5"
=#

using Pkg
Pkg.status()

println("hello world")
```
and ion will parse the #=ion ... block to automatically setup an environment, running this will print

> ion run script.jl
>
> Status `~/.local/bin/env/env-3506815430/Project.toml`
>
>    [7876af07] Example v0.5.3
>
> hello world

You probably want to ask why this is not a Project.toml or Manifest.toml embedded inside the script like Pluto notebook. Like many other similar implementations, we want this part editable and readable since it will directly appear in your script. Thus if you can install a package with a specific version in Julia REPL by only providing the version number and name, you should be typing the same information in your script too!

We also have an alternative mode letting you specify UUID, git revision, URL, path, etc, e.g

```julia
# !/usr/bin/env ion run
#=ion
Example = {version="0.5", url="https://github.com/Roger-luo/Example.jl"}
=#

using Pkg
Pkg.status()

println("hello world")
```
On the other hand, there might be cases where we want a script to be never changed, which is something I'm thinking to have a release mode script environment specification that is similar to Pluto notebook that has a complete Manifest.toml and Project.toml inside the script.

#### A Local Environment by Default

As for normal scripts, many Julia users want `julia --project` to be the default, which is something I advocate too, and I have been using alias jp="julia --project" in my terminal for years. Though this has been a safety concern for julia compiler binary, it is not a concern for a developer tool that only runs locally. So for a script without the #=ion dependencies, ion run is equivalent to the following (and it forwards Julia compiler flags like --threads etc. if you specify it)
```
command 			equivalent to
ion run 			julia --project
ion run script.jl 	julia --project script.jl
```

#### Clone Julia packages

Have you gotten annoyed that cloning a Julia package using git ends up in a folder with xxxx.jl by default?

Have you been opening a browser, searching the package, copying the package git URL, then cloning the package somewhere?

Have you tried to let dev command use your own directory instead of .julia/dev ?

Have you cloned a Julia package, ready to contribute to it, but realize you need to fork it and change remote origin to remote upstream and add your own fork?

Now ion clone handles all above with just one line! if you try `ion clone Example` it will look for the registered URL and try to clone it, and because you don't seem to have access to this repo, we will ask if you want to fork it and if you say yes, we will do it for you. No opening browser is needed!

#### Create a new package with pre-defined templates

Have you ever typed the same project configuration again and again interactively with PkgTemplates?

Have you ever typed the wrong option in the interactive mode with PkgTemplates and had to start over entirely?

The `ion new` command is here to help: we create an entirely new templating system based on PkgTemplates but with serialization in TOML.

The following is a project template for a small project:

```TOML
name="project"
description = "A project description"

[readme]
[project_file]
[src_dir]
[tests]
```

and the following is a project template for research packages:

```TOML
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

Most importantly you can save your own custom configuration and share it with people! Maybe your company's internal packages need a custom README template and LICENSE? Create your own template.toml with corresponding components and share it with your colleagues instead of asking them to do it interactively! Check examples in our [template registry here](https://github.com/Roger-luo/ion-templates)!

#### What's next?

I'm hoping to have self-update support like juliaup in the future, but I haven't had the time to work it out, for other features. I'm also thinking about JuliaFormatter and JET integration similar to cargo fmt and cargo clippy, but I haven't decided on how integration is supported yet.

Last, please feel free to open issues on bug reports, feature requests, or contributing PRs!

### Command Quick Reference:

NB: sub-commands are indicated with bullet points and options are indicated with bullet points plus short or long flags (ie. -f or --flag).
Arguments are listed in square brackets, such as [URL]. A positional arg which is capable of handling multiple arguments is listed with ellipses following the name (eg. [EXAMPLE...]).

Optional arguments are listed with a `?` before the name; generally speaking, if required arguments are not provided, Ion will ask for them.


| Commands                     | Description                                                  |
| ---------------------------- | ------------------------------------------------------------ |
| new [PATH]                   | Create a new package                                         |
| clone [URL] [PATH]           | Clone a package from URL or registry                         |
| release [VERSION] [PATH]     | release a new version of a package                           |
| bump [VERSION] [PATH]        | bump the version of a package                                |
| summon [PATH]                | summon JuliaRegistrator to register the package              |
| run [PATH]                   | Run a script, or start a REPL if no script is given          |
| script update/rm/repl [PATH] | script tools                                                 |
| add  [PACKAGE...]            | Add dependencies to current environment                      |
| remove [PACKAGE...]          | Remove dependencies in the current environment [aliases: rm] |
| develop [PACKAGE]            | Develop packages in the current environment [aliases: dev]   |
| free [PACKAGE]               | Free pinned packages in the current environment              |
| gc                           | garbage collect packages not used for a significant time     |
| precompile [PACKAGE...]      | Precompile all packages in the current environment           |
| status                       | Show the status of the current environment [aliases: st]     |
| update  [PACKAGE...]         | Update the current environment [aliases: up]                 |
| why [PACKAGE]                | show why a package is installed                              |
| template list/update/inspect | template management                                          |
| completions [SHELL]          | generate shell completion scripts                            |
| auth login/logout            | manage Github authentication                                 |

---

#### Command List
  ion
- auth
  - login
  - logout
- clone [?options] [URL] [PATH]
  - --registry [REGISTRY]
- release [?options] [VERSION] [PATH]
  - --branch | -b [?BRANCH]
  - --registry [?REGISTRY]
  - --no-prompt
  - --no-commit
  - --no-report
  - --skip-note
- summon [?options] [PATH]
  - --branch | -b
  - --no-prompt
  - --skip-note
- bump [?options] [VERSION] [PATH]
  - --branch | -b [BRANCH]
  - --no-prompt
  - --no-commit
  - --no-report
  - --registry [REGISTRY]
- new [?options] [PATH]
  - --list
  - --force | -f
  - --no-interactive
  - --template | -t [TEMPLATE]
- run [?options] [PATH]
  - --sysimage | -J [PATH]
  - --threads | -t [?NUMBER_OF_THREADS]
  - --procs | -p [?NUMBER_OF_PROCESSES]
  - --color [?OPT]
  - --verbose | -v
- script
    - update [?options] [PATH]
      - --verbose | -v
    - rm [PATH]
    - repl [PATH]
- add [?options] [PACKAGE...]
  - --global | -g
- develop | dev [?options] [PACKAGE]
  - --global | -g
  - --all
  - --version | -v
- free [?options] [PACKAGE]
  - --global | -g
- gc [?options]
  - --global | -g
- precompile [?options] [PACKAGE...]
  - --strict
  - --global | -g
- remove | rm [?options] [PACKAGE...]
  - --global | -g
- status | st [?options]
  - --outdated
  - --no-diff
  - --manifest
  - --global | -g
- update | up [?options] [PACKAGE...]
  - --global | -g
- why [?options] [PACKAGE]
  - --global | -g
- completions [SHELL]
- template
  - list
  - update
  - inspect [?TEMPLATE_NAME]
    - --all
    - --verbose | -v


## License

MIT License
