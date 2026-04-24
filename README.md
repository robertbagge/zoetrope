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

Requires [ffmpeg](https://ffmpeg.org/) with libwebp support if you want WebP output
(`brew install ffmpeg-full` on macOS; the standard `ffmpeg` package on Ubuntu includes it).

```sh
cargo install --path .
```

## Usage

```sh
zoetrope demo.mov                           # → demo.gif (medium quality)
zoetrope demo.mp4 -q high                   # mp4/webm/mkv/avi also supported
zoetrope demo.mov -F webp                   # → demo.webp (2-5x smaller)
zoetrope demo.mov --start 5s --end 12s      # trim to a 7-second clip
zoetrope demo.mov --start 1:30 --duration 10s
zoetrope demo.mov --speed 2                 # 2x speedup
zoetrope demo.mov --playback boomerang      # forward then reverse
zoetrope demo.mov -q high --fps 24          # preset + manual override
zoetrope demo.mov --max-size 500kb          # shrink until it fits
zoetrope demo.mov --for slack               # platform preset with auto-fit
zoetrope demo.mov --force                   # overwrite existing output
```

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

For WebP output, a single ffmpeg pass encodes directly with libwebp.
