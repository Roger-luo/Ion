# Changelog

All notable changes to this project will be documented in this file.

### Added

- Standardize binary skill interface under `self` subcommand (46472f2)

### CI

- Add CI workflow with cargo-nextest for parallel test execution (89aee89)

### Refactored

- Use workspace dependencies for shared crates (7543aab)
- Rename ionlib crate to ionem (c83ccd3)
- Extract ionlib crate for downstream binary skill developers (5f3d65c)

### Added

- Show hint when configuring codex target (377e00e)

### Added

- Add documentation website with Astro Starlight (df7eac4)

### Fixed

- Show source registry in search TUI detail panel and fix owner display (12bfc7f)
- Call add directly from search TUI and update docs with accurate specs (69e108b)

### Fixed

- Use semver comparison for self-update version check (87086de)

### Documentation

- Add Why Ion section comparing with npx skills (8dc6e00)

### Fixed

- Remove redundant merged_options computation in install (ae669e2)

### Refactored

- Use ProjectContext in run command (d77a927)
- Remove duplicated register_in_registry from migrate (d286244)
- Add ProjectContext::ensure_builtin_skill() method (ca9691e)
- Remove UpdateContext in favor of SkillInstaller (2b79674)
- Move resolve_entry from Manifest to SkillEntry (07872c6)
- Add SkillSource::display_name() method (ffc3564)
- Add SkillSource constructors and builder methods (d813c83)
- Use Finding's Serialize impl in validate command (283d303)
- Use install_shared helpers in add command (ff8b092)
- Use install_shared helpers in install command (c3fe425)
- Create install_shared module with shared install helpers (92f8e41)

### Style

- Remove unused imports after merge (a62a6a2)
- Apply cargo fmt (e366115)

### Added

- Make --json work for all commands (b3d5244)

### Documentation

- Clarify breaking change conventions and skills-dir default in AGENTS.md (711c7de)

### Added

- Render config panel inline instead of full-screen (a115957)
- Add contextual hints to interactive config panel (dc1621b)
- Show skills-dir in interactive config panel (4985499)
- Install.sh detects conflicting ion installs and offers alias (4282756)
- Detect package manager and block self update when managed externally (6bb5e29)

### Documentation

- Document local skill creation and eject in builtin SKILL.md (d9f4e14)

### Fixed

- Clear leftover lines after quitting inline config panel (dafb715)
- Show default config values in hint color, not green (354456b)
- Change skills-dir default to .agents/skills and show defaults in config (0224a1b)

### Refactored

- Two-column config panel with hints on the right (a0d0f28)

### Added

- Auto-refresh global ion-cli SKILL.md on every invocation (e68712c)
- Generate SKILL.md from minijinja template with real JSON examples (c3710f3)
- Add minijinja template for SKILL.md generation (d733a9d)

### Build

- Add minijinja as dev-dependency for template-based SKILL.md generation (ee797b2)

### Fixed

- Make cache gc example static in SKILL.md template (6c45ed2)
- Guard println in remove command for --json mode (9054515)

### Refactored

- Generate SKILL.md at build time, remove from git tracking (b1256aa)
- Generate all JSON examples programmatically, eliminate hand-typed JSON from template (ef7e17e)

### Fixed

- Prevent circular symlink for binary skills and auto-deploy ion-cli (58da614)
- Sort releases by semver in install.sh instead of taking first (d87ea0d)

### Added

- Detect existing install in install.sh and add --version flag (04ce4cf)
- Add `ion self uninstall` command (7d23dff)

### Fixed

- Handle old ion binaries without --version in install.sh (ba4baac)

### Added

- Enhance migrate command with JSON interface, leftover handling, and gitignore support (2d6895d)
- Hint about available updates on every invocation (a61a3c7)
- Embed SKILL.md as built-in ion-cli skill (8217503)
- Add global --json flag for agent/script interface (d8cfe64)
- Prompt for shell completion setup in install script ([#72](https://github.com/Roger-luo/Ion/pull/72)) (f3602dc)
- Prompt user to star GitHub repos after install ([#71](https://github.com/Roger-luo/Ion/pull/71)) (4b86ff6)
- Add ion completion command ([#66](https://github.com/Roger-luo/Ion/pull/66)) (ae6c751)
- Allow specifying version in install script (caed683)
- Add one-line install script (e5fa685)
- Add TUI target selection widget for ion init (e2fb9b6)
- Batch validation prompts for skill collections (63b54be)
- Add ion update command for upgrading binary skills (cd2c0fa)
- Clean up binary files when removing binary skills (29b282b)
- Add ion run command for executing binary skills (43a23c0)
- Add --bin flag to ion add for binary CLI skills (a8d0a80)
- Add Binary variant to SourceType and SkillSource (71457da)
- Add colored output to CLI commands (25e7b39)
- Ion remove fuzzy-matches against skill names and sources (b8643c8)
- Confirm before removing skills, add -y/--yes to skip prompt (71362c9)
- Ion remove accepts source prefix to remove all matching skills (a4417c6)
- Show hint about ion init when no targets configured (f78ece6)
- Implement ion init interactive mode with legacy file rename (b361fea)
- Implement ion init flag mode (1b318d2)
- Add ion init command skeleton with known-targets lookup (ba8fc64)
- Add ion gc command for cleaning stale repos (7f7f334)
- Wire global registry into add/install/remove (92ec72d)
- Add ion link command for local skill directories (2045b15)
- Wire per-skill gitignore into add/install/remove (572053e)
- Implement ion new --collection for multi-skill projects (6485f7e)
- Add --collection flag to ion new CLI (9a27140)
- Add ion init command with SKILL.md scaffolding (59d155a)
- Add ion validate command with recursive skill scanning (78d6bb9)
- Prompt on validation warnings during install flows (a4afa91)
- Add validate module with SkillChecker trait and severity types (39027bc)
- Grouped search results with skills.sh integration (6bd1601)
- Wire up two-column TUI in search interactive mode (68b486a)
- Add search TUI renderer with two-column layout (8b3c62d)
- Add search TUI event handler (920e763)
- Add SearchApp state struct for search TUI (3b7de30)
- Show GitHub stars, repo description, and SKILL.md description in search results (9b64c71)
- Add --verbose flag to search command for debug logging (ac1c528)
- Add ion search command with cascade, parallel, and interactive modes (0782639)
- Wire up interactive TUI main loop for config command (5b00f65)
- Add TUI app state, rendering, and event handling for config command (68c28d9)
- Add config command with get/set/list subcommands (404af4b)
- Add ratatui and crossterm dependencies for config TUI (da5bb15)
- Integrate global config into remove command (23ea99d)
- Integrate global config into add command (cc5d02b)
- Integrate global config into install command (5e610a8)
- Prompt to add managed directories to .gitignore after install (f871440)
- Add `ion migrate` command to import skills from skills-lock.json (0956870)
- Wire up all CLI commands (add, remove, install, list, info) (623cd68)
- Add skill installer with local path support, copy, and uninstall (3c15ace)
- Add manifest writing with toml_edit for add/remove operations (156515c)
- Set up workspace with ion-skill crate and CLI skeleton (823c023)

### CI

- Merge release binary builds into release-plz workflow (fa5f456)
- Add release-plz for automated version bumps and changelog (3d6a6b2)

### Documentation

- Add SKILL.md for agent CLI interface and update README (8975a85)
- Add JSON agent interface implementation plan (2404caa)
- Add JSON agent interface design (d633ef5)
- Update AGENTS.md with local skills, self-update, cache, and release conventions (dbb39eb)
- Document update system in AGENTS.md (d1deede)
- Add implementation plan for non-binary skill updates (b3b74c9)
- Add design for extending ion update to non-binary skills (e449af4)
- Implementation plan for binary CLI skills Phase 2 (98faa32)
- Implementation plan for binary CLI skills Phase 1 (2330085)
- Design for binary CLI skills support (38be7e6)
- Implementation plan for ion init and target discoverability (0dac963)
- Design for ion init and target discoverability (34203a2)
- Add implementation plan for symlink deployment and per-skill gitignore (89dbe50)
- Add design for symlink-based deployment and per-skill gitignore (69a9800)
- Add implementation plan for ion new command (ded5315)
- Add design for ion new command with --collection support (321e5ef)
- Add design and implementation plan for ion init command (44f06c1)
- Add skill validation expansion implementation plan (1e96565)
- Add skill validation expansion design (1a25d59)
- Add skill validation implementation plan (862eca6)
- Add skill validation design (d1948cd)
- Add codebase simplification implementation plan (ba3f303)
- Add codebase simplification design (88c2832)
- Add search TUI implementation plan (14dfb6a)
- Add search TUI two-column layout design (3f79145)
- Add search command implementation plan (2332fe9)
- Add search command design (02f7eea)
- Add config command implementation plan (7f3181d)
- Add config command design (e0a2e25)
- Add implementation plan for global configuration (61a41fa)
- Add design for global configuration (c88ca63)
- Add implementation plan for symlink-based skill installation (e271829)
- Add design for symlink-based skill installation (e337a7d)

### Fixed

- Add workflow_dispatch trigger for manual release builds (1393dd6)
- Trigger release builds on release event instead of tag push (560b35c)
- Align release workflow and self-update with release-plz tag format (96138a1)
- Silently exit on cancel in ion init interactive mode (d6be1aa)
- Wire up TUI multi-select widget for ion init (eb55703)
- Rename ion.toml/ion.lock to Ion.toml/Ion.lock and refine security checker (86199bb)
- Address code review issues in binary skill implementation (32be864)
- Address code quality issues in search TUI (b52d456)
- Address code review issues - URL encoding, shell injection, timeouts, error handling (ae70792)
- Improve gitignore prompt with default hint (24586a3)
- Address clippy warnings from symlink migration (92d287e)

### Refactored

- Rewrite update command to use Updater trait dispatch (573e411)
- Rename ion init to ion new (6aaea87)
- Split search.rs into module directory and eliminate duplication (bb0e6dd)
- Introduce SkillInstaller struct to encapsulate install/uninstall logic (b307478)
- Introduce ProjectContext to eliminate path boilerplate across commands (5f083c7)
- Extract TUI terminal lifecycle helper to deduplicate setup/teardown (c1d8cd4)
- Deduplicate config list section printing (0a8df33)
- Extract shared wrap_text utility into tui::util (c88e67b)

### Testing

- Add integration tests for non-binary skill updates (bea0eea)
- Add Phase 2 integration tests for binary lifecycle (0c8d366)
- Add integration tests for binary skill data model roundtrip (1b87c11)
- Add failing tests for ion new --collection (462a142)
- Add search integration tests (fa74007)
- Add integration tests for config command (bef1b9a)
- Update integration tests for symlink-based installation (d896429)
- Add integration tests for add, remove, install, list, and info (3623b19)

### Agent

- Add CLAUDE.md delegate to AGENT.md (2c83ba4)

### Deps

- Add reqwest and dialoguer for search command (246b694)

### Added

- Enhance migrate command with JSON interface, leftover handling, and gitignore support (2d6895d)

### Added

- Hint about available updates on every invocation (a61a3c7)
- Embed SKILL.md as built-in ion-cli skill (8217503)
- Add global --json flag for agent/script interface (d8cfe64)
- Prompt for shell completion setup in install script ([#72](https://github.com/Roger-luo/Ion/pull/72)) (f3602dc)
- Prompt user to star GitHub repos after install ([#71](https://github.com/Roger-luo/Ion/pull/71)) (4b86ff6)
- Add ion completion command ([#66](https://github.com/Roger-luo/Ion/pull/66)) (ae6c751)
- Allow specifying version in install script (caed683)
- Add one-line install script (e5fa685)
- Add TUI target selection widget for ion init (e2fb9b6)
- Batch validation prompts for skill collections (63b54be)
- Add ion update command for upgrading binary skills (cd2c0fa)
- Clean up binary files when removing binary skills (29b282b)
- Add ion run command for executing binary skills (43a23c0)
- Add --bin flag to ion add for binary CLI skills (a8d0a80)
- Add Binary variant to SourceType and SkillSource (71457da)
- Add colored output to CLI commands (25e7b39)
- Ion remove fuzzy-matches against skill names and sources (b8643c8)
- Confirm before removing skills, add -y/--yes to skip prompt (71362c9)
- Ion remove accepts source prefix to remove all matching skills (a4417c6)
- Show hint about ion init when no targets configured (f78ece6)
- Implement ion init interactive mode with legacy file rename (b361fea)
- Implement ion init flag mode (1b318d2)
- Add ion init command skeleton with known-targets lookup (ba8fc64)
- Add ion gc command for cleaning stale repos (7f7f334)
- Wire global registry into add/install/remove (92ec72d)
- Add ion link command for local skill directories (2045b15)
- Wire per-skill gitignore into add/install/remove (572053e)
- Implement ion new --collection for multi-skill projects (6485f7e)
- Add --collection flag to ion new CLI (9a27140)
- Add ion init command with SKILL.md scaffolding (59d155a)
- Add ion validate command with recursive skill scanning (78d6bb9)
- Prompt on validation warnings during install flows (a4afa91)
- Add validate module with SkillChecker trait and severity types (39027bc)
- Grouped search results with skills.sh integration (6bd1601)
- Wire up two-column TUI in search interactive mode (68b486a)
- Add search TUI renderer with two-column layout (8b3c62d)
- Add search TUI event handler (920e763)
- Add SearchApp state struct for search TUI (3b7de30)
- Show GitHub stars, repo description, and SKILL.md description in search results (9b64c71)
- Add --verbose flag to search command for debug logging (ac1c528)
- Add ion search command with cascade, parallel, and interactive modes (0782639)
- Wire up interactive TUI main loop for config command (5b00f65)
- Add TUI app state, rendering, and event handling for config command (68c28d9)
- Add config command with get/set/list subcommands (404af4b)
- Add ratatui and crossterm dependencies for config TUI (da5bb15)
- Integrate global config into remove command (23ea99d)
- Integrate global config into add command (cc5d02b)
- Integrate global config into install command (5e610a8)
- Prompt to add managed directories to .gitignore after install (f871440)
- Add `ion migrate` command to import skills from skills-lock.json (0956870)
- Wire up all CLI commands (add, remove, install, list, info) (623cd68)
- Add skill installer with local path support, copy, and uninstall (3c15ace)
- Add manifest writing with toml_edit for add/remove operations (156515c)
- Set up workspace with ion-skill crate and CLI skeleton (823c023)

### CI

- Merge release binary builds into release-plz workflow (fa5f456)
- Add release-plz for automated version bumps and changelog (3d6a6b2)

### Documentation

- Add SKILL.md for agent CLI interface and update README (8975a85)
- Add JSON agent interface implementation plan (2404caa)
- Add JSON agent interface design (d633ef5)
- Update AGENTS.md with local skills, self-update, cache, and release conventions (dbb39eb)
- Document update system in AGENTS.md (d1deede)
- Add implementation plan for non-binary skill updates (b3b74c9)
- Add design for extending ion update to non-binary skills (e449af4)
- Implementation plan for binary CLI skills Phase 2 (98faa32)
- Implementation plan for binary CLI skills Phase 1 (2330085)
- Design for binary CLI skills support (38be7e6)
- Implementation plan for ion init and target discoverability (0dac963)
- Design for ion init and target discoverability (34203a2)
- Add implementation plan for symlink deployment and per-skill gitignore (89dbe50)
- Add design for symlink-based deployment and per-skill gitignore (69a9800)
- Add implementation plan for ion new command (ded5315)
- Add design for ion new command with --collection support (321e5ef)
- Add design and implementation plan for ion init command (44f06c1)
- Add skill validation expansion implementation plan (1e96565)
- Add skill validation expansion design (1a25d59)
- Add skill validation implementation plan (862eca6)
- Add skill validation design (d1948cd)
- Add codebase simplification implementation plan (ba3f303)
- Add codebase simplification design (88c2832)
- Add search TUI implementation plan (14dfb6a)
- Add search TUI two-column layout design (3f79145)
- Add search command implementation plan (2332fe9)
- Add search command design (02f7eea)
- Add config command implementation plan (7f3181d)
- Add config command design (e0a2e25)
- Add implementation plan for global configuration (61a41fa)
- Add design for global configuration (c88ca63)
- Add implementation plan for symlink-based skill installation (e271829)
- Add design for symlink-based skill installation (e337a7d)

### Fixed

- Add workflow_dispatch trigger for manual release builds (1393dd6)
- Trigger release builds on release event instead of tag push (560b35c)
- Align release workflow and self-update with release-plz tag format (96138a1)
- Silently exit on cancel in ion init interactive mode (d6be1aa)
- Wire up TUI multi-select widget for ion init (eb55703)
- Rename ion.toml/ion.lock to Ion.toml/Ion.lock and refine security checker (86199bb)
- Address code review issues in binary skill implementation (32be864)
- Address code quality issues in search TUI (b52d456)
- Address code review issues - URL encoding, shell injection, timeouts, error handling (ae70792)
- Improve gitignore prompt with default hint (24586a3)
- Address clippy warnings from symlink migration (92d287e)

### Refactored

- Rewrite update command to use Updater trait dispatch (573e411)
- Rename ion init to ion new (6aaea87)
- Split search.rs into module directory and eliminate duplication (bb0e6dd)
- Introduce SkillInstaller struct to encapsulate install/uninstall logic (b307478)
- Introduce ProjectContext to eliminate path boilerplate across commands (5f083c7)
- Extract TUI terminal lifecycle helper to deduplicate setup/teardown (c1d8cd4)
- Deduplicate config list section printing (0a8df33)
- Extract shared wrap_text utility into tui::util (c88e67b)

### Testing

- Add integration tests for non-binary skill updates (bea0eea)
- Add Phase 2 integration tests for binary lifecycle (0c8d366)
- Add integration tests for binary skill data model roundtrip (1b87c11)
- Add failing tests for ion new --collection (462a142)
- Add search integration tests (fa74007)
- Add integration tests for config command (bef1b9a)
- Update integration tests for symlink-based installation (d896429)
- Add integration tests for add, remove, install, list, and info (3623b19)

### Agent

- Add CLAUDE.md delegate to AGENT.md (2c83ba4)

### Deps

- Add reqwest and dialoguer for search command (246b694)

### Added

- Hint about available updates on every invocation (a61a3c7)
- Embed SKILL.md as built-in ion-cli skill (8217503)
- Add global --json flag for agent/script interface (d8cfe64)
- Prompt for shell completion setup in install script ([#72](https://github.com/Roger-luo/Ion/pull/72)) (f3602dc)
- Prompt user to star GitHub repos after install ([#71](https://github.com/Roger-luo/Ion/pull/71)) (4b86ff6)
- Add ion completion command ([#66](https://github.com/Roger-luo/Ion/pull/66)) (ae6c751)
- Allow specifying version in install script (caed683)
- Add one-line install script (e5fa685)
- Add TUI target selection widget for ion init (e2fb9b6)
- Batch validation prompts for skill collections (63b54be)
- Add ion update command for upgrading binary skills (cd2c0fa)
- Clean up binary files when removing binary skills (29b282b)
- Add ion run command for executing binary skills (43a23c0)
- Add --bin flag to ion add for binary CLI skills (a8d0a80)
- Add Binary variant to SourceType and SkillSource (71457da)
- Add colored output to CLI commands (25e7b39)
- Ion remove fuzzy-matches against skill names and sources (b8643c8)
- Confirm before removing skills, add -y/--yes to skip prompt (71362c9)
- Ion remove accepts source prefix to remove all matching skills (a4417c6)
- Show hint about ion init when no targets configured (f78ece6)
- Implement ion init interactive mode with legacy file rename (b361fea)
- Implement ion init flag mode (1b318d2)
- Add ion init command skeleton with known-targets lookup (ba8fc64)
- Add ion gc command for cleaning stale repos (7f7f334)
- Wire global registry into add/install/remove (92ec72d)
- Add ion link command for local skill directories (2045b15)
- Wire per-skill gitignore into add/install/remove (572053e)
- Implement ion new --collection for multi-skill projects (6485f7e)
- Add --collection flag to ion new CLI (9a27140)
- Add ion init command with SKILL.md scaffolding (59d155a)
- Add ion validate command with recursive skill scanning (78d6bb9)
- Prompt on validation warnings during install flows (a4afa91)
- Add validate module with SkillChecker trait and severity types (39027bc)
- Grouped search results with skills.sh integration (6bd1601)
- Wire up two-column TUI in search interactive mode (68b486a)
- Add search TUI renderer with two-column layout (8b3c62d)
- Add search TUI event handler (920e763)
- Add SearchApp state struct for search TUI (3b7de30)
- Show GitHub stars, repo description, and SKILL.md description in search results (9b64c71)
- Add --verbose flag to search command for debug logging (ac1c528)
- Add ion search command with cascade, parallel, and interactive modes (0782639)
- Wire up interactive TUI main loop for config command (5b00f65)
- Add TUI app state, rendering, and event handling for config command (68c28d9)
- Add config command with get/set/list subcommands (404af4b)
- Add ratatui and crossterm dependencies for config TUI (da5bb15)
- Integrate global config into remove command (23ea99d)
- Integrate global config into add command (cc5d02b)
- Integrate global config into install command (5e610a8)
- Prompt to add managed directories to .gitignore after install (f871440)
- Add `ion migrate` command to import skills from skills-lock.json (0956870)
- Wire up all CLI commands (add, remove, install, list, info) (623cd68)
- Add skill installer with local path support, copy, and uninstall (3c15ace)
- Add manifest writing with toml_edit for add/remove operations (156515c)
- Set up workspace with ion-skill crate and CLI skeleton (823c023)

### CI

- Merge release binary builds into release-plz workflow (fa5f456)
- Add release-plz for automated version bumps and changelog (3d6a6b2)

### Documentation

- Add SKILL.md for agent CLI interface and update README (8975a85)
- Add JSON agent interface implementation plan (2404caa)
- Add JSON agent interface design (d633ef5)
- Update AGENTS.md with local skills, self-update, cache, and release conventions (dbb39eb)
- Document update system in AGENTS.md (d1deede)
- Add implementation plan for non-binary skill updates (b3b74c9)
- Add design for extending ion update to non-binary skills (e449af4)
- Implementation plan for binary CLI skills Phase 2 (98faa32)
- Implementation plan for binary CLI skills Phase 1 (2330085)
- Design for binary CLI skills support (38be7e6)
- Implementation plan for ion init and target discoverability (0dac963)
- Design for ion init and target discoverability (34203a2)
- Add implementation plan for symlink deployment and per-skill gitignore (89dbe50)
- Add design for symlink-based deployment and per-skill gitignore (69a9800)
- Add implementation plan for ion new command (ded5315)
- Add design for ion new command with --collection support (321e5ef)
- Add design and implementation plan for ion init command (44f06c1)
- Add skill validation expansion implementation plan (1e96565)
- Add skill validation expansion design (1a25d59)
- Add skill validation implementation plan (862eca6)
- Add skill validation design (d1948cd)
- Add codebase simplification implementation plan (ba3f303)
- Add codebase simplification design (88c2832)
- Add search TUI implementation plan (14dfb6a)
- Add search TUI two-column layout design (3f79145)
- Add search command implementation plan (2332fe9)
- Add search command design (02f7eea)
- Add config command implementation plan (7f3181d)
- Add config command design (e0a2e25)
- Add implementation plan for global configuration (61a41fa)
- Add design for global configuration (c88ca63)
- Add implementation plan for symlink-based skill installation (e271829)
- Add design for symlink-based skill installation (e337a7d)

### Fixed

- Add workflow_dispatch trigger for manual release builds (1393dd6)
- Trigger release builds on release event instead of tag push (560b35c)
- Align release workflow and self-update with release-plz tag format (96138a1)
- Silently exit on cancel in ion init interactive mode (d6be1aa)
- Wire up TUI multi-select widget for ion init (eb55703)
- Rename ion.toml/ion.lock to Ion.toml/Ion.lock and refine security checker (86199bb)
- Address code review issues in binary skill implementation (32be864)
- Address code quality issues in search TUI (b52d456)
- Address code review issues - URL encoding, shell injection, timeouts, error handling (ae70792)
- Improve gitignore prompt with default hint (24586a3)
- Address clippy warnings from symlink migration (92d287e)

### Refactored

- Rewrite update command to use Updater trait dispatch (573e411)
- Rename ion init to ion new (6aaea87)
- Split search.rs into module directory and eliminate duplication (bb0e6dd)
- Introduce SkillInstaller struct to encapsulate install/uninstall logic (b307478)
- Introduce ProjectContext to eliminate path boilerplate across commands (5f083c7)
- Extract TUI terminal lifecycle helper to deduplicate setup/teardown (c1d8cd4)
- Deduplicate config list section printing (0a8df33)
- Extract shared wrap_text utility into tui::util (c88e67b)

### Testing

- Add integration tests for non-binary skill updates (bea0eea)
- Add Phase 2 integration tests for binary lifecycle (0c8d366)
- Add integration tests for binary skill data model roundtrip (1b87c11)
- Add failing tests for ion new --collection (462a142)
- Add search integration tests (fa74007)
- Add integration tests for config command (bef1b9a)
- Update integration tests for symlink-based installation (d896429)
- Add integration tests for add, remove, install, list, and info (3623b19)

### Agent

- Add CLAUDE.md delegate to AGENT.md (2c83ba4)

### Deps

- Add reqwest and dialoguer for search command (246b694)

### Added

- Hint about available updates on every invocation (a61a3c7)

### Added

- Embed SKILL.md as built-in ion-cli skill (8217503)
- Add global --json flag for agent/script interface (d8cfe64)

### Documentation

- Add SKILL.md for agent CLI interface and update README (8975a85)
- Add JSON agent interface implementation plan (2404caa)
- Add JSON agent interface design (d633ef5)

### Added

- Prompt for shell completion setup in install script ([#72](https://github.com/Roger-luo/Ion/pull/72)) (f3602dc)
- Prompt user to star GitHub repos after install ([#71](https://github.com/Roger-luo/Ion/pull/71)) (4b86ff6)

### Added

- Add ion completion command ([#66](https://github.com/Roger-luo/Ion/pull/66)) (ae6c751)

### CI

- Merge release binary builds into release-plz workflow (fa5f456)

### Added

- Allow specifying version in install script (caed683)
- Add one-line install script (e5fa685)

### Fixed

- Add workflow_dispatch trigger for manual release builds (1393dd6)
- Trigger release builds on release event instead of tag push (560b35c)

### Fixed

- Align release workflow and self-update with release-plz tag format (96138a1)

### Added

- Add TUI target selection widget for ion init (e2fb9b6)
- Batch validation prompts for skill collections (63b54be)
- Add ion update command for upgrading binary skills (cd2c0fa)
- Clean up binary files when removing binary skills (29b282b)
- Add ion run command for executing binary skills (43a23c0)
- Add --bin flag to ion add for binary CLI skills (a8d0a80)
- Add Binary variant to SourceType and SkillSource (71457da)
- Add colored output to CLI commands (25e7b39)
- Ion remove fuzzy-matches against skill names and sources (b8643c8)
- Confirm before removing skills, add -y/--yes to skip prompt (71362c9)
- Ion remove accepts source prefix to remove all matching skills (a4417c6)
- Show hint about ion init when no targets configured (f78ece6)
- Implement ion init interactive mode with legacy file rename (b361fea)
- Implement ion init flag mode (1b318d2)
- Add ion init command skeleton with known-targets lookup (ba8fc64)
- Add ion gc command for cleaning stale repos (7f7f334)
- Wire global registry into add/install/remove (92ec72d)
- Add ion link command for local skill directories (2045b15)
- Wire per-skill gitignore into add/install/remove (572053e)
- Implement ion new --collection for multi-skill projects (6485f7e)
- Add --collection flag to ion new CLI (9a27140)
- Add ion init command with SKILL.md scaffolding (59d155a)
- Add ion validate command with recursive skill scanning (78d6bb9)
- Prompt on validation warnings during install flows (a4afa91)
- Add validate module with SkillChecker trait and severity types (39027bc)
- Grouped search results with skills.sh integration (6bd1601)
- Wire up two-column TUI in search interactive mode (68b486a)
- Add search TUI renderer with two-column layout (8b3c62d)
- Add search TUI event handler (920e763)
- Add SearchApp state struct for search TUI (3b7de30)
- Show GitHub stars, repo description, and SKILL.md description in search results (9b64c71)
- Add --verbose flag to search command for debug logging (ac1c528)
- Add ion search command with cascade, parallel, and interactive modes (0782639)
- Wire up interactive TUI main loop for config command (5b00f65)
- Add TUI app state, rendering, and event handling for config command (68c28d9)
- Add config command with get/set/list subcommands (404af4b)
- Add ratatui and crossterm dependencies for config TUI (da5bb15)
- Integrate global config into remove command (23ea99d)
- Integrate global config into add command (cc5d02b)
- Integrate global config into install command (5e610a8)
- Prompt to add managed directories to .gitignore after install (f871440)
- Add `ion migrate` command to import skills from skills-lock.json (0956870)
- Wire up all CLI commands (add, remove, install, list, info) (623cd68)
- Add skill installer with local path support, copy, and uninstall (3c15ace)
- Add manifest writing with toml_edit for add/remove operations (156515c)
- Set up workspace with ion-skill crate and CLI skeleton (823c023)

### CI

- Add release-plz for automated version bumps and changelog (3d6a6b2)

### Documentation

- Document update system in AGENTS.md (d1deede)
- Add implementation plan for non-binary skill updates (b3b74c9)
- Add design for extending ion update to non-binary skills (e449af4)
- Implementation plan for binary CLI skills Phase 2 (98faa32)
- Implementation plan for binary CLI skills Phase 1 (2330085)
- Design for binary CLI skills support (38be7e6)
- Implementation plan for ion init and target discoverability (0dac963)
- Design for ion init and target discoverability (34203a2)
- Add implementation plan for symlink deployment and per-skill gitignore (89dbe50)
- Add design for symlink-based deployment and per-skill gitignore (69a9800)
- Add implementation plan for ion new command (ded5315)
- Add design for ion new command with --collection support (321e5ef)
- Add design and implementation plan for ion init command (44f06c1)
- Add skill validation expansion implementation plan (1e96565)
- Add skill validation expansion design (1a25d59)
- Add skill validation implementation plan (862eca6)
- Add skill validation design (d1948cd)
- Add codebase simplification implementation plan (ba3f303)
- Add codebase simplification design (88c2832)
- Add search TUI implementation plan (14dfb6a)
- Add search TUI two-column layout design (3f79145)
- Add search command implementation plan (2332fe9)
- Add search command design (02f7eea)
- Add config command implementation plan (7f3181d)
- Add config command design (e0a2e25)
- Add implementation plan for global configuration (61a41fa)
- Add design for global configuration (c88ca63)
- Add implementation plan for symlink-based skill installation (e271829)
- Add design for symlink-based skill installation (e337a7d)

### Fixed

- Silently exit on cancel in ion init interactive mode (d6be1aa)
- Wire up TUI multi-select widget for ion init (eb55703)
- Rename ion.toml/ion.lock to Ion.toml/Ion.lock and refine security checker (86199bb)
- Address code review issues in binary skill implementation (32be864)
- Address code quality issues in search TUI (b52d456)
- Address code review issues - URL encoding, shell injection, timeouts, error handling (ae70792)
- Improve gitignore prompt with default hint (24586a3)
- Address clippy warnings from symlink migration (92d287e)

### Refactored

- Rewrite update command to use Updater trait dispatch (573e411)
- Rename ion init to ion new (6aaea87)
- Split search.rs into module directory and eliminate duplication (bb0e6dd)
- Introduce SkillInstaller struct to encapsulate install/uninstall logic (b307478)
- Introduce ProjectContext to eliminate path boilerplate across commands (5f083c7)
- Extract TUI terminal lifecycle helper to deduplicate setup/teardown (c1d8cd4)
- Deduplicate config list section printing (0a8df33)
- Extract shared wrap_text utility into tui::util (c88e67b)

### Testing

- Add integration tests for non-binary skill updates (bea0eea)
- Add Phase 2 integration tests for binary lifecycle (0c8d366)
- Add integration tests for binary skill data model roundtrip (1b87c11)
- Add failing tests for ion new --collection (462a142)
- Add search integration tests (fa74007)
- Add integration tests for config command (bef1b9a)
- Update integration tests for symlink-based installation (d896429)
- Add integration tests for add, remove, install, list, and info (3623b19)

### Agent

- Add CLAUDE.md delegate to AGENT.md (2c83ba4)

### Deps

- Add reqwest and dialoguer for search command (246b694)
