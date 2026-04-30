# Zoetrope

> **zoetrope** */ˈzoʊ.ɪ.troʊp/* — A 19th-century optical device consisting of a spinning cylinder with slits and a strip of sequential images inside. When spun, the images blur together into the illusion of motion. Invented in 1834 by William George Horner, the zoetrope was one of the first forms of animation — and arguably, the world's first gif.

---

A fast CLI that converts screen recordings into high-quality GIFs (via [gifski](https://gif.ski)) or animated WebP (via ffmpeg/libwebp).

## Install

### Homebrew (recommended)

```sh
brew tap robertbagge/tap
brew install zoetrope
```

This installs ffmpeg automatically as a dependency.

### From source

Requires [ffmpeg](https://ffmpeg.org/) (any standard build — libwebp is bundled
into the binary, so the stock `brew install ffmpeg` and `apt install ffmpeg`
packages are sufficient for both GIF and WebP output).

```sh
cargo install --path crates/zoetrope-cli
```

## Usage

`zoetrope --help` prints the same categorised examples. A quick tour:

### Basics

```sh
zoetrope demo.mov                           # → demo.gif (medium quality)
zoetrope demo.mp4                           # mp4/webm/mkv/avi also supported
zoetrope demo.mov -o clip.gif               # custom output filename
zoetrope demo.mov -F webp                   # → demo.webp (2-5x smaller)
zoetrope demo.mov --force                   # overwrite existing output
```

### Quality and size

```sh
zoetrope demo.mov -q high                   # 1440px, 15fps preset
zoetrope demo.mov --width 640               # override width only
zoetrope demo.mov --fps 20                  # override frame rate only
zoetrope demo.mov -q high --fps 24          # preset + manual override
zoetrope demo.mov --max-size 500kb          # shrink iteratively to fit
zoetrope demo.mov --for slack               # platform preset with auto-fit
zoetrope demo.mov --for slack --fps 15      # preset + manual override
```

### Trim, speed, playback

```sh
zoetrope demo.mov --start 5s --end 12s      # 7-second clip
zoetrope demo.mov --start 1:30 --duration 10s
zoetrope demo.mov --end 10s                 # first 10 seconds
zoetrope demo.mov --speed 2                 # 2x speedup
zoetrope demo.mov --speed 0.5               # slow motion
zoetrope demo.mov --playback reverse
zoetrope demo.mov --playback boomerang      # forward then reverse
```

### Batch

Pass multiple inputs to convert them in one invocation. By default each output
lands next to its input; `--output-dir <DIR>` collects them in one place
(creating the directory if needed).

```sh
zoetrope *.mov                              # each → .gif next to input
zoetrope a.mov b.mp4 c.webm                 # mixed formats
zoetrope *.mov --output-dir ./gifs/         # collect outputs in one dir
zoetrope *.mov --for slack --output-dir ./slack/
```

All other flags (`--for`, `--width`, `--speed`, `-q`, etc.) apply uniformly to
every file. `-o/--output` is single-input only — use `--output-dir` for batch.

## Smart Sizing

`--max-size <SIZE>` iteratively shrinks the output until it fits. Accepts
`5mb`, `500kb`, `2gb`, or raw bytes. Sizes are decimal (1 mb = 1,000,000 bytes)
to match how GitHub, Slack, and Discord document their upload limits. Capped
at 5 attempts.

`--for <PLATFORM>` applies a platform preset (dimensions + size limit + encoder
quality) and enables auto-fit:

| Platform  | Size cap  | Width  | FPS | Encoder q |
|-----------|-----------|--------|-----|-----------|
| `slack`   | 5 MB      | 480px  | 10  | 80        |
| `github`  | 10 MB     | 960px  | 12  | 85        |
| `discord` | 8 MB      | 640px  | 12  | 80        |
| `twitter` | 5 MB      | 480px  | 10  | 80        |
| `email`   | 500 KB    | 320px  | 8   | 75        |

Platform presets lock the format to GIF. `--fps`, `--width`, `--max-size`, and
`-q` all override the preset when supplied.

## Quality Presets

| Preset | Width  | FPS | Best for |
|--------|--------|-----|----------|
| `low`     | 480px  | 8  | Slack, quick shares |
| `medium`  | 960px  | 12 | GitHub PRs, docs |
| `high`    | 1440px | 15 | Presentations, LinkedIn |
| `ultra`   | 2048px | 24 | Demo reels, high-fidelity |

`--fps` and `--width` override the preset when you need fine control.

## How It Works

For GIF output, zoetrope runs ffmpeg to decode, trim, speed-adjust, scale, and
extract PNG frames, then hands them to [gifski](https://gif.ski) — which gives
each frame its own palette with temporal dithering. The result is sharper and
closer to the source than ffmpeg's single-palette output.

For WebP output, ffmpeg extracts the same PNG frames and the
[webp-animation](https://crates.io/crates/webp-animation) crate (which
statically embeds libwebp) assembles the animated WebP in-process. This means
the binary needs no special ffmpeg build — the stock package works.
