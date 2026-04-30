pub const SUPPORTED_INPUT_FORMATS: &[&str] = &["mov", "mp4", "webm", "mkv", "avi"];

pub struct QualitySettings {
    pub width: u32,
    pub fps: u32,
    pub gifski_quality: u8,
    pub webp_quality: u8,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Quality {
    /// 480px, 8fps — small files for chat
    Low,
    /// 960px, 12fps — default; good for GitHub PRs and docs
    Medium,
    /// 1440px, 15fps — presentations and LinkedIn
    High,
    /// 2048px, 24fps — demo reels, high-fidelity
    Ultra,
}

impl Quality {
    pub fn settings(&self) -> QualitySettings {
        match self {
            Quality::Low => QualitySettings {
                width: 480,
                fps: 8,
                gifski_quality: 60,
                webp_quality: 60,
            },
            Quality::Medium => QualitySettings {
                width: 960,
                fps: 12,
                gifski_quality: 80,
                webp_quality: 80,
            },
            Quality::High => QualitySettings {
                width: 1440,
                fps: 15,
                gifski_quality: 95,
                webp_quality: 90,
            },
            Quality::Ultra => QualitySettings {
                width: 2048,
                fps: 24,
                gifski_quality: 100,
                webp_quality: 95,
            },
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Format {
    /// GIF — universal compatibility, larger files
    Gif,
    /// Animated WebP — 2-5x smaller files, 97% browser support
    Webp,
}

impl Format {
    pub fn extension(&self) -> &'static str {
        match self {
            Format::Gif => "gif",
            Format::Webp => "webp",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Playback {
    /// Standard forward playback
    Normal,
    /// Play the video backwards
    Reverse,
    /// Forward then reverse (ping-pong loop) — doubles frame count
    Boomerang,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Platform {
    /// ≤5 MB, 480px, 10fps — tight for chat
    Slack,
    /// ≤10 MB, 960px, 12fps — PR/issue attachments
    Github,
    /// ≤8 MB, 640px, 12fps
    Discord,
    /// ≤5 MB, 480px, 10fps
    Twitter,
    /// ≤500 KB, 320px, 8fps — inline-friendly
    Email,
}

pub struct PlatformSettings {
    pub max_size: u64,
    pub width: u32,
    pub fps: u32,
    pub gifski_quality: u8,
}

impl Platform {
    pub fn settings(&self) -> PlatformSettings {
        match self {
            Platform::Slack => PlatformSettings {
                max_size: 5_000_000,
                width: 480,
                fps: 10,
                gifski_quality: 80,
            },
            Platform::Github => PlatformSettings {
                max_size: 10_000_000,
                width: 960,
                fps: 12,
                gifski_quality: 85,
            },
            Platform::Discord => PlatformSettings {
                max_size: 8_000_000,
                width: 640,
                fps: 12,
                gifski_quality: 80,
            },
            Platform::Twitter => PlatformSettings {
                max_size: 5_000_000,
                width: 480,
                fps: 10,
                gifski_quality: 80,
            },
            Platform::Email => PlatformSettings {
                max_size: 500_000,
                width: 320,
                fps: 8,
                gifski_quality: 75,
            },
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Platform::Slack => "slack",
            Platform::Github => "github",
            Platform::Discord => "discord",
            Platform::Twitter => "twitter",
            Platform::Email => "email",
        }
    }
}
