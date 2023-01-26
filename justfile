build target="aarch64-apple-darwin":
    cargo build --bin ion --release --target {{target}}

tarball target="aarch64-apple-darwin":
    #!/usr/bin/env bash
    DIST="target/{{target}}/dist"
    mkdir -p $DIST
    cp target/{{target}}/release/ion $DIST/ion
    cp -r resources $DIST/resources
    cd target/{{target}} && tar -czf ion-{{target}}.tar.gz dist
    ARCHIVE="target/{{target}}/ion-{{target}}.tar.gz"
    echo "::set-output name=archive::$ARCHIVE"

delete-release tag:
    gh release delete {{tag}} -y
    git push --delete origin {{tag}}

release tag:
    gh release create {{tag}} -t {{tag}} --generate-notes -p


[macos]
install:
    cargo build --bin ion --release
    mkdir -p $HOME/.local/bin
    cp target/release/ion $HOME/.local/bin
    mkdir -p $HOME/Library/Application\ Support/ion
    cp -r resources $HOME/Library/Application\ Support/ion/resources
