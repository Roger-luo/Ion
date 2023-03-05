---
sidebar_position: 1
---

# Intro

Everything in Ion is optimized for terminal UX, including its name - ion is the easiest-to-type name that I find not used by other popular CLI tools.

This is still at quite an early stage, but I have been using it for a month myself, so I'd like to share this tool with the community. There will be more detailed documentation and a website set up in the future.

## Commands available

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

## Command List

NB: sub-commands are indicated with bullet points and options are indicated with bullet points plus short or long flags (ie. -f or --flag).
Arguments are listed in square brackets, such as [URL]. A positional arg which is capable of handling multiple arguments is listed with ellipses following the name (eg. [EXAMPLE...]).

Optional arguments are listed with a `?` before the name; generally speaking, if required arguments are not provided, Ion will ask for them.

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
