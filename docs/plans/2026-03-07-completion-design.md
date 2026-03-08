# Shell Completion Design

## Summary

Add `ion completion <shell>` to generate shell auto-completion scripts. Uses `clap_complete` to generate completions for bash, zsh, fish, elvish, and powershell.

## Usage

```
ion completion bash
ion completion zsh
ion completion fish
ion completion elvish
ion completion powershell
```

Output goes to stdout. Users redirect to the appropriate shell config location:

```bash
ion completion bash > ~/.local/share/bash-completion/completions/ion
ion completion zsh > ~/.zfunc/_ion
ion completion fish > ~/.config/fish/completions/ion.fish
```

## Implementation

1. Add `clap_complete = "4"` to root `Cargo.toml`
2. Add `Completion { shell: clap_complete::Shell }` variant to `Commands` enum — `Shell` implements `clap::ValueEnum` so it works as a positional arg
3. Create `src/commands/completion.rs` with `pub fn run(shell, cmd)` that calls `clap_complete::generate()` writing to stdout
4. Wire up in `main.rs` match arm, passing `Cli::command()` to get the clap `Command` object

## Testing

Integration test verifying each shell variant exits 0 and produces non-empty output.
