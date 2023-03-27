---

---

# Running Julia with Ion

## Run Julia with local environment `--project` by default

As for normal scripts, many Julia users want `julia --project` to be the default, which is something I advocate too, and I have been using `alias jp="julia --project"` in my terminal for years. Though this has been a safety concern for julia compiler binary, it is not a concern for a developer tool that only runs locally. So for a script without the `#=ion` dependencies (introduced in next section), `ion run` is equivalent to the following (and it forwards Julia compiler flags like `--threads` etc. if you specify it)

| command | equivalent to |
| --- | --- |
| `ion run` | `julia --project` |
| `ion run script.jl` | `julia --project script.jl` |

**and with shell auto-completion if you have set it up!**

## Run a standalone script with dependencies

This is a feature that is supported in many other languages: Single-file scripts that download their dependencies. The lack of this feature has been a gripe from users in the Julia community; Ion solves this pain point with its new script system.

You can write the following in your Julia script

```julia
# !/usr/bin/env ion run
#=ion
Example = "0.5"
=#

using Pkg
Pkg.status()

println("hello world")
```

and ion will parse the `#=ion ...` block to automatically setup an environment, running this will print

```sh
ion run script.jl

Status `~/.local/bin/env/env-3506815430/Project.toml`
    [7876af07] Example v0.5.3

hello world
```

You probably want to ask why this is not a `Project.toml` or `Manifest.toml` embedded inside the script like Pluto notebook. Like many other similar implementations, we want this part **editable and readable** since it will directly appear in your script. Thus if you can install a package with a specific version in Julia REPL by only providing the version number and name, you should be typing the same information in your script too!

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

On the other hand, there might be cases where we want a script to be never changed, which is something I'm thinking to have a release mode script environment specification that is similar to Pluto notebook that has a complete `Manifest.toml` and `Project.toml` inside the script.
