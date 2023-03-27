---
sidebar_position: 3
---

# Releasing a new version with Ion

This has been one of the most frequently used features personally in my previous Julia version, and in this new rust version, I have rewritten the whole thing in a much cleaner and modular fashion.

- Have you tired of wanting to bump a patch version, but forgetting it's already
  bumped in main and only noticing it's a wrong patch version after the General registry complains to you after 3min?
- Have you been tired of opening your editor, changing the version in Project.toml
  manually then opening a browser to summon JuliaRegistrator but having the bot name typed wrong?

With ion release you only need one line to do all these within seconds! e.g

```sh
ion release patch
```

will automatically bump a patch version based on your current version in main and your registered version. If it's not a continuous version number, ion will warn you and ask if you want to continue; if your current version number is already valid, ion will ask if you want to release that instead - and Ion will also summon JuliaRegistrator directly without asking you to open the browser.

## Releases

Custom release pipeline with summon and bump:

We now also provide bump and summon commands to allow you to do more customization, e.g managing large mono-repo that contains many other packages or maybe you want to pack up some artifact before summon JuliaRegistrator. You can combine these two commands with [`just`](https://github.com/casey/just) (recommended) or `make`, like what I do here.

![ion-release-gif-demo](/img/ion.gif)
