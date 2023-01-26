build target:
    cargo build --bins --release --target {{target}}

tarball:
    tar -czf $(NAME)-$(VERSION).tar.gz target/release/$(NAME)
