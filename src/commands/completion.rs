use std::io;

pub fn run(shell: clap_complete::Shell, mut cmd: clap::Command) {
    clap_complete::generate(shell, &mut cmd, "ion", &mut io::stdout());
}
