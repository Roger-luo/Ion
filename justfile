build target="aarch64-apple-darwin":
    cargo build --bin ion --release -F bin --target {{target}}

tarball target="aarch64-apple-darwin":
    #!/usr/bin/env bash
    DIST="target/{{target}}/dist"
    VERSION="$(cargo xtask version)"
    NAME="ion-{{VERSION}}-{{target}}"
    mkdir -p $DIST
    mkdir -p $DIST/bin
    cp target/{{target}}/release/ion $DIST/bin/ion
    cd target/{{target}} && tar -czf {{NAME}}.tar.gz dist
    ARCHIVE="target/{{target}}/{{NAME}}.tar.gz"
    echo "::set-output name=archive::$ARCHIVE"

delete-release tag:
    gh release delete v{{tag}} -y
    git push --delete origin v{{tag}}

release tag:
    #!/usr/bin/env bash
    cargo xtask release {{tag}}
    VERSION="$(cargo xtask version)"
    git add Cargo.toml
    git diff --quiet Cargo.toml && git diff --staged --quiet || git commit -m "Bump version to {{VERSION}}"
    git pull origin main
    git push origin main
    gh release create v{{VERSION}} -t v{{VERSION}} --generate-notes

[macos]
install prefix="$HOME/.local":
    cargo build --bin ion --release -F bin
    mkdir -p {{prefix}}/bin
    cp target/release/ion {{prefix}}/bin

[linux]
install prefix="$HOME/.local":
    #!/usr/bin/env bash
    cargo build --bin ion --release -F bin
    mkdir -p {{prefix}}/bin
    cp target/release/ion {{prefix}}/bin/ion
