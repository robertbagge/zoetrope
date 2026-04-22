# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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
