# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.0](https://github.com/robertbagge/zoetrope/compare/v0.1.0...v0.2.0) (2026-04-22)


### Features

* migrate from release-plz to release-please ([9212cc9](https://github.com/robertbagge/zoetrope/commit/9212cc99bd7abb497a3f9b82257559b5607310ba))


### Bug Fixes

* add contents write permission to build job for release uploads ([08221aa](https://github.com/robertbagge/zoetrope/commit/08221aaaa7aae4dc7ec4b5547670cc04ac041026))
* add publish = false to Cargo.toml to prevent crates.io publish ([a91455c](https://github.com/robertbagge/zoetrope/commit/a91455c596ee68e9958dd0fa74d8bd35da2e8d64))
* improve ffmpeg check to detect broken installations ([d903536](https://github.com/robertbagge/zoetrope/commit/d90353626e5113d6e0c63a3851018ec14203d03d))
* use git_only mode for version detection ([cf4aba4](https://github.com/robertbagge/zoetrope/commit/cf4aba4d1f8160e71d2478f4220a5ac2767d63e7))

## [Unreleased]

## [0.1.1](https://github.com/robertbagge/zoetrope/compare/v0.1.0...v0.1.1) - 2026-04-22

### Fixed

- use git_only mode for version detection
- add contents write permission to build job for release uploads
- improve ffmpeg check to detect broken installations

### Other

- enable verbose logging for release-pr debugging

## [0.1.0](https://github.com/robertbagge/zoetrope/releases/tag/v0.1.0) - 2026-04-22

### Added

- add CI release pipeline with homebrew tap auto-update
- initial commit — mov to gif converter

### Fixed

- pass all job outputs via env vars in build steps
- harden CI security and update readme
- disable crates.io publish, use git tags for versioning
- extract homebrew formula to template file
- switch to release-pr + release two-step flow
- enable direct releases and fix workflow output parsing

### Other

- add readme with zoetrope trivia and usage guide
- Initial commit
