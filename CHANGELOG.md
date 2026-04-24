# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.4.0](https://github.com/robertbagge/zoetrope/compare/v0.3.0...v0.4.0) (2026-04-24)


### Features

* --for platform presets (slack, github, discord, twitter, email) ([73cf749](https://github.com/robertbagge/zoetrope/commit/73cf74954f4a34dbdb1e66f3d3d173481791f015))
* --max-size flag with iterative fit loop ([6f3ca59](https://github.com/robertbagge/zoetrope/commit/6f3ca59d12cd91108ec2fa46d89fe1a1f7a741cf))
* chunk 1 — gifski pipeline, webp, trim, speed, playback ([52ef14c](https://github.com/robertbagge/zoetrope/commit/52ef14ca57710518db84207342c6c5365edfdafb))
* chunk 1 — gifski pipeline, webp, trim, speed, playback ([1731a2d](https://github.com/robertbagge/zoetrope/commit/1731a2d8f17ae98a2cec25df95917c85c0a8814c))
* progress bars via indicatif + README updates ([7f5f7a1](https://github.com/robertbagge/zoetrope/commit/7f5f7a14fedeab8f9ecc932833c93bf0c99a65d2))

## [0.3.0](https://github.com/robertbagge/zoetrope/compare/v0.2.1...v0.3.0) (2026-04-22)


### Features

* add ultra quality preset (2K/24fps) ([3e3fc72](https://github.com/robertbagge/zoetrope/commit/3e3fc722c746b3a4085e38a05d7ed91d897f9795))

## [0.2.1](https://github.com/robertbagge/zoetrope/compare/v0.2.0...v0.2.1) (2026-04-22)


### Bug Fixes

* drop Intel mac support, ARM only ([975b5c2](https://github.com/robertbagge/zoetrope/commit/975b5c2740eef1242eb4cf2228e873299e1942eb))

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
