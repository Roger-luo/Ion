---
sidebar_position: 4
---

# Cloning the repository of a package

- Have you gotten annoyed that cloning a Julia package using git ends up in a folder with `xxxx.jl` by default?
- Have you been opening a browser, searching the package, copying the package git URL, then cloning the package somewhere?
- Have you tried to let dev command use your own directory instead of `.julia/dev` ?
- Have you cloned a Julia package, ready to contribute to it, but realize you need to fork it and change remote origin to remote upstream and add your own fork?

Now ion clone handles all above with just one line! if you try `ion clone Example` it will look for the registered URL and try to clone it, and because you don't seem to have access to this repo, we will ask if you want to fork it and if you say yes, we will do it for you. **No opening browser is needed!**
