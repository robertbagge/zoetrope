# Zoetrope

> **zoetrope** */ˈzoʊ.ɪ.troʊp/* — A 19th-century optical device consisting of a spinning cylinder with slits and a strip of sequential images inside. When spun, the images blur together into the illusion of motion. Invented in 1834 by William George Horner, the zoetrope was one of the first forms of animation — and arguably, the world's first gif.

---

A fast CLI that converts `.mov` screen recordings into high-quality gifs using ffmpeg's two-pass palette technique.

## Install

Requires [ffmpeg](https://ffmpeg.org/) (`brew install ffmpeg` on macOS).

```sh
cargo install --path .
```

## Usage

```sh
zoetrope recording.mov                     # → recording.gif (medium quality)
zoetrope recording.mov -q high             # 1440px, 15fps
zoetrope recording.mov -q low              # 480px, 8fps — small files
zoetrope recording.mov -o demo.gif         # custom output path
zoetrope recording.mov -q high --fps 24    # preset + manual override
zoetrope recording.mov --force             # overwrite existing output
```

## Quality Presets

| Preset | Width | FPS | Best for |
|--------|-------|-----|----------|
| `low` | 480px | 8 | Slack, quick shares |
| `medium` | 960px | 12 | GitHub PRs, docs |
| `high` | 1440px | 15 | Presentations, LinkedIn |

`--fps` and `--width` flags override the preset values when you need fine control.

## How It Works

Single-pass gif encoding uses a generic 256-color palette, which produces muddy colors and visible banding. Zoetrope runs ffmpeg twice:

1. **Pass 1** — Analyzes the video and generates an optimized 256-color palette tuned to the actual content
2. **Pass 2** — Encodes the gif using that palette with Bayer dithering for smooth gradients

The difference is significant, especially for screen recordings with UI elements and text.
