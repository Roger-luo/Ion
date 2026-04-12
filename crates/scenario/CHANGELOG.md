# Changelog

All notable changes to this project will be documented in this file.

### Added

- Add expect_not() to assert text absence ([#126](https://github.com/Roger-luo/Ion/pull/126)) (1258498)
- Add visible_screen() to get terminal display state ([#127](https://github.com/Roger-luo/Ion/pull/127)) (73aebf3)
- Add send_key() helper for common terminal keys ([#125](https://github.com/Roger-luo/Ion/pull/125)) (c84d62d)

## 0.1.2

### Fixed

- Fix repository URL to point to the ion monorepo
- Publish full API to crates.io (v0.1.0 was a stub with no exports)

## 0.1.1

### Added

- Add Scenario::project() integration method (f71e2da)
- Implement template loading, variable validation, and rendering (15519c7)
- Implement Project::empty() with file and dir support (931b5c1)
- Implement template.toml manifest parsing (1c4474b)
- Scaffold project fixture modules and error variants (dca17d9)

### Testing

- Add template error case tests (688b980)
- Add build_in tests (c85ffea)
- Add symlink creation tests (26cabc1)
- Add override and extra file tests (1224a5d)
- Add path mapping tests (6bf849b)
- Add file filtering tests for optional/include/exclude (0055299)
