build target="aarch64-apple-darwin":
    cargo build --bin ion --release -F bin --target {{target}}

tarball target="aarch64-apple-darwin":
    #!/usr/bin/env bash
    DIST="target/{{target}}/dist"
    mkdir -p $DIST
    mkdir -p $DIST/bin
    cp target/{{target}}/release/ion $DIST/bin/ion
    cd target/{{target}} && tar -czf ion-{{target}}.tar.gz dist
    ARCHIVE="target/{{target}}/ion-{{target}}.tar.gz"
    echo "::set-output name=archive::$ARCHIVE"

delete-release tag:
    gh release delete v{{tag}} -y
    git push --delete origin v{{tag}}

release tag:
    cargo xtask bump {{tag}}
    git add Cargo.toml
    git diff --quiet Cargo.toml && git diff --staged --quiet || git commit -m "Bump version to {{tag}}"
    git pull origin main
    git push origin main
    gh release create v{{tag}} -t v{{tag}} --generate-notes

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
