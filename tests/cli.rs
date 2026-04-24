use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;

mod common;
use common::{decode_gif, fixture, libwebp_available, mov_fixture};

fn zoetrope() -> Command {
    Command::cargo_bin("zoetrope").expect("binary not built")
}

// ─── Input formats ──────────────────────────────────────────────────────────

#[test]
fn test_mov_input_produces_gif() {
    let dir = TempDir::new().unwrap();
    let input = mov_fixture(dir.path());
    let output = dir.path().join("in.gif");

    zoetrope().arg(&input).assert().success();

    assert!(output.exists(), "output gif should exist");
    let size = std::fs::metadata(&output).unwrap().len();
    assert!(size > 0, "output gif should be non-empty");
}

#[test]
fn test_mp4_input_produces_gif() {
    assert_format_produces_gif("mp4");
}

#[test]
fn test_webm_input_produces_gif() {
    assert_format_produces_gif("webm");
}

#[test]
fn test_mkv_input_produces_gif() {
    assert_format_produces_gif("mkv");
}

#[test]
fn test_avi_input_produces_gif() {
    assert_format_produces_gif("avi");
}

fn assert_format_produces_gif(ext: &str) {
    let dir = TempDir::new().unwrap();
    let input = fixture(dir.path(), "in", ext);
    let output = dir.path().join("in.gif");

    zoetrope().arg(&input).assert().success();

    assert!(output.exists(), "output gif should exist for {ext}");
    let size = std::fs::metadata(&output).unwrap().len();
    assert!(size > 0, "output gif should be non-empty for {ext}");
}

#[test]
fn test_missing_input_errors() {
    let dir = TempDir::new().unwrap();
    let input = dir.path().join("nope.mov");

    zoetrope()
        .arg(&input)
        .assert()
        .failure()
        .stderr(predicate::str::contains("file not found"));
}

#[test]
fn test_unsupported_extension_errors() {
    let dir = TempDir::new().unwrap();
    let input = dir.path().join("image.png");
    std::fs::write(&input, b"").unwrap();

    zoetrope()
        .arg(&input)
        .assert()
        .failure()
        .stderr(predicate::str::contains("input must be one of:"))
        .stderr(predicate::str::contains("mov"))
        .stderr(predicate::str::contains("mp4"))
        .stderr(predicate::str::contains("webm"));
}

// ─── Quality presets ────────────────────────────────────────────────────────

#[test]
fn test_quality_preset_low() {
    let dir = TempDir::new().unwrap();
    let input = mov_fixture(dir.path());

    zoetrope()
        .arg(&input)
        .args(["-q", "low"])
        .assert()
        .success();

    let (w, _, _) = decode_gif(&dir.path().join("in.gif"));
    assert_eq!(w, 480, "low preset width");
}

#[test]
fn test_quality_preset_medium() {
    let dir = TempDir::new().unwrap();
    let input = mov_fixture(dir.path());

    zoetrope()
        .arg(&input)
        .args(["-q", "medium"])
        .assert()
        .success();

    let (w, _, _) = decode_gif(&dir.path().join("in.gif"));
    assert_eq!(w, 960, "medium preset width");
}

#[test]
fn test_quality_preset_high() {
    let dir = TempDir::new().unwrap();
    let input = mov_fixture(dir.path());

    zoetrope()
        .arg(&input)
        .args(["-q", "high"])
        .assert()
        .success();

    let (w, _, _) = decode_gif(&dir.path().join("in.gif"));
    assert_eq!(w, 1440, "high preset width");
}

#[test]
fn test_quality_preset_ultra() {
    let dir = TempDir::new().unwrap();
    let input = mov_fixture(dir.path());

    zoetrope()
        .arg(&input)
        .args(["-q", "ultra"])
        .assert()
        .success();

    let (w, _, _) = decode_gif(&dir.path().join("in.gif"));
    assert_eq!(w, 2048, "ultra preset width");
}

// ─── Overrides ──────────────────────────────────────────────────────────────

#[test]
fn test_fps_override() {
    let dir = TempDir::new().unwrap();
    let input = mov_fixture(dir.path());

    // 2s fixture at 10fps → ~20 frames
    zoetrope()
        .arg(&input)
        .args(["--fps", "10"])
        .assert()
        .success();

    let (_, _, frames) = decode_gif(&dir.path().join("in.gif"));
    assert!(
        (18..=22).contains(&frames),
        "expected ~20 frames at 10fps over 2s, got {frames}"
    );
}

#[test]
fn test_width_override() {
    let dir = TempDir::new().unwrap();
    let input = mov_fixture(dir.path());

    zoetrope()
        .arg(&input)
        .args(["--width", "320"])
        .assert()
        .success();

    let (w, _, _) = decode_gif(&dir.path().join("in.gif"));
    assert_eq!(w, 320, "width override");
}

#[test]
fn test_output_path_override() {
    let dir = TempDir::new().unwrap();
    let input = mov_fixture(dir.path());
    let custom = dir.path().join("custom-name.gif");

    zoetrope()
        .arg(&input)
        .args(["-o".as_ref(), custom.as_os_str()])
        .assert()
        .success();

    assert!(custom.exists(), "custom output path should exist");
    assert!(
        !dir.path().join("in.gif").exists(),
        "default output should NOT exist when -o is provided"
    );
}

#[test]
fn test_no_force_errors_on_existing_output() {
    let dir = TempDir::new().unwrap();
    let input = mov_fixture(dir.path());
    let output = dir.path().join("in.gif");
    std::fs::write(&output, b"existing").unwrap();

    zoetrope()
        .arg(&input)
        .assert()
        .failure()
        .stderr(predicate::str::contains("already exists"));
}

#[test]
fn test_force_overwrites() {
    let dir = TempDir::new().unwrap();
    let input = mov_fixture(dir.path());
    let output = dir.path().join("in.gif");
    std::fs::write(&output, b"existing").unwrap();

    zoetrope().arg(&input).arg("--force").assert().success();

    let size = std::fs::metadata(&output).unwrap().len();
    assert!(
        size > b"existing".len() as u64,
        "output should be overwritten with a real gif"
    );
}

// ─── Trimming ───────────────────────────────────────────────────────────────

#[test]
fn test_start_trim_reduces_duration() {
    // 2s fixture @ 12fps (medium) = ~24 frames.
    // --start 1s cuts first second → ~12 frames.
    let dir = TempDir::new().unwrap();
    let input = mov_fixture(dir.path());

    zoetrope()
        .arg(&input)
        .args(["--start", "1s"])
        .assert()
        .success();

    let (_, _, frames) = decode_gif(&dir.path().join("in.gif"));
    assert!(
        (8..=16).contains(&frames),
        "expected ~12 frames after 1s trim, got {frames}"
    );
}

#[test]
fn test_end_trim_reduces_duration() {
    // --end 1s keeps only first second → ~12 frames.
    let dir = TempDir::new().unwrap();
    let input = mov_fixture(dir.path());

    zoetrope()
        .arg(&input)
        .args(["--end", "1s"])
        .assert()
        .success();

    let (_, _, frames) = decode_gif(&dir.path().join("in.gif"));
    assert!(
        (8..=16).contains(&frames),
        "expected ~12 frames with --end 1s, got {frames}"
    );
}

#[test]
fn test_duration_flag() {
    // --start 0s --duration 1s → 1s of video → ~12 frames.
    let dir = TempDir::new().unwrap();
    let input = mov_fixture(dir.path());

    zoetrope()
        .arg(&input)
        .args(["--start", "0s", "--duration", "1s"])
        .assert()
        .success();

    let (_, _, frames) = decode_gif(&dir.path().join("in.gif"));
    assert!(
        (8..=16).contains(&frames),
        "expected ~12 frames over 1s duration, got {frames}"
    );
}

#[test]
fn test_end_and_duration_conflict_errors() {
    let dir = TempDir::new().unwrap();
    let input = mov_fixture(dir.path());

    zoetrope()
        .arg(&input)
        .args(["--end", "1s", "--duration", "1s"])
        .assert()
        .failure();
}

// ─── WebP ───────────────────────────────────────────────────────────────────

#[test]
fn test_webp_output_produces_valid_file() {
    if !libwebp_available() {
        eprintln!("skipping: ffmpeg built without libwebp");
        return;
    }

    let dir = TempDir::new().unwrap();
    let input = mov_fixture(dir.path());
    let output = dir.path().join("in.webp");

    zoetrope()
        .arg(&input)
        .args(["-F", "webp"])
        .assert()
        .success();

    assert!(output.exists(), "webp output should exist");
    let bytes = std::fs::read(&output).unwrap();
    assert!(bytes.len() >= 12, "webp too small to contain magic header");
    assert_eq!(&bytes[0..4], b"RIFF", "webp should start with RIFF");
    assert_eq!(&bytes[8..12], b"WEBP", "webp should contain WEBP magic");
}

#[test]
fn test_webp_default_extension() {
    if !libwebp_available() {
        eprintln!("skipping: ffmpeg built without libwebp");
        return;
    }

    let dir = TempDir::new().unwrap();
    let input = mov_fixture(dir.path());

    zoetrope()
        .arg(&input)
        .args(["-F", "webp"])
        .assert()
        .success();

    assert!(
        dir.path().join("in.webp").exists(),
        "webp default output should exist"
    );
    assert!(
        !dir.path().join("in.gif").exists(),
        "gif should not be created when -F webp is used"
    );
}

#[test]
fn test_webp_missing_encoder_errors_cleanly() {
    if libwebp_available() {
        eprintln!("skipping: libwebp is available, so this error path is unreachable");
        return;
    }

    let dir = TempDir::new().unwrap();
    let input = mov_fixture(dir.path());

    zoetrope()
        .arg(&input)
        .args(["-F", "webp"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("libwebp"));
}

// ─── Speed ──────────────────────────────────────────────────────────────────

#[test]
fn test_speed_2x_halves_frames() {
    // 2s fixture @ 12fps baseline = ~24 frames. At 2x → ~12 frames.
    let dir = TempDir::new().unwrap();
    let input = mov_fixture(dir.path());

    zoetrope()
        .arg(&input)
        .args(["--speed", "2"])
        .assert()
        .success();

    let (_, _, frames) = decode_gif(&dir.path().join("in.gif"));
    assert!(
        (8..=16).contains(&frames),
        "expected ~12 frames at 2x speed, got {frames}"
    );
}

#[test]
fn test_speed_half_increases_frames() {
    // 2s fixture @ 12fps baseline = ~24 frames. At 0.5x → ~48 frames.
    let dir = TempDir::new().unwrap();
    let input = mov_fixture(dir.path());

    zoetrope()
        .arg(&input)
        .args(["--speed", "0.5"])
        .assert()
        .success();

    let (_, _, frames) = decode_gif(&dir.path().join("in.gif"));
    assert!(
        (40..=56).contains(&frames),
        "expected ~48 frames at 0.5x speed, got {frames}"
    );
}

#[test]
fn test_speed_rejects_zero() {
    let dir = TempDir::new().unwrap();
    let input = mov_fixture(dir.path());

    zoetrope()
        .arg(&input)
        .args(["--speed", "0"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("--speed"));
}

// ─── Playback modes ─────────────────────────────────────────────────────────

#[test]
fn test_reverse_runs() {
    let dir = TempDir::new().unwrap();
    let input = mov_fixture(dir.path());
    let output = dir.path().join("in.gif");

    zoetrope()
        .arg(&input)
        .args(["--playback", "reverse"])
        .assert()
        .success();

    let size = std::fs::metadata(&output).unwrap().len();
    assert!(size > 0, "reverse output should be non-empty");
}

#[test]
fn test_boomerang_doubles_frames() {
    // 2s fixture @ 12fps baseline = ~24 frames. Boomerang → ~48 frames.
    let dir = TempDir::new().unwrap();
    let input = mov_fixture(dir.path());

    zoetrope()
        .arg(&input)
        .args(["--playback", "boomerang"])
        .assert()
        .success();

    let (_, _, frames) = decode_gif(&dir.path().join("in.gif"));
    assert!(
        (40..=56).contains(&frames),
        "expected ~48 frames for boomerang (2x baseline), got {frames}"
    );
}

// ─── Format inference from --output extension ──────────────────────────────

#[test]
fn test_output_extension_infers_webp_format() {
    if !libwebp_available() {
        eprintln!("skipping: ffmpeg built without libwebp");
        return;
    }

    let dir = TempDir::new().unwrap();
    let input = mov_fixture(dir.path());
    let custom = dir.path().join("custom.webp");

    zoetrope()
        .arg(&input)
        .args(["-o".as_ref(), custom.as_os_str()])
        .assert()
        .success();

    let bytes = std::fs::read(&custom).unwrap();
    assert!(bytes.len() >= 12, "output too small");
    assert_eq!(
        &bytes[0..4],
        b"RIFF",
        "output should be webp when -o is .webp"
    );
    assert_eq!(
        &bytes[8..12],
        b"WEBP",
        "output should be webp when -o is .webp"
    );
}

#[test]
fn test_format_and_output_extension_mismatch_errors() {
    let dir = TempDir::new().unwrap();
    let input = mov_fixture(dir.path());
    let custom = dir.path().join("out.webp");

    zoetrope()
        .arg(&input)
        .args(["-F", "gif"])
        .args(["-o".as_ref(), custom.as_os_str()])
        .assert()
        .failure()
        .stderr(predicate::str::contains("conflicts"))
        .stderr(predicate::str::contains(".webp"));
}

// ─── Cross-feature integration ──────────────────────────────────────────────

#[test]
fn test_kitchen_sink_trim_speed_boomerang_width() {
    // 2s fixture @ 30fps.
    // --start 0.5s → 1.5s remaining
    // --speed 2   → 0.75s
    // boomerang   → 1.5s
    // --fps 10    → ~15 frames at 400px
    let dir = TempDir::new().unwrap();
    let input = mov_fixture(dir.path());

    zoetrope()
        .arg(&input)
        .args([
            "--start",
            "0.5s",
            "--speed",
            "2",
            "--playback",
            "boomerang",
            "--fps",
            "10",
            "--width",
            "400",
        ])
        .assert()
        .success();

    let (w, _, frames) = decode_gif(&dir.path().join("in.gif"));
    assert_eq!(w, 400, "width should honor --width override");
    assert!(
        (10..=20).contains(&frames),
        "expected ~15 frames (1.5s @ 10fps boomeranged), got {frames}"
    );
}

// ─── Fixture helper sanity check ────────────────────────────────────────────

#[test]
fn test_fixture_helper_generates_other_formats() {
    let dir = TempDir::new().unwrap();
    for ext in ["mp4", "webm", "mkv", "avi"] {
        let path = fixture(dir.path(), "probe", ext);
        assert!(path.exists(), "{ext} fixture should exist");
        let size = std::fs::metadata(&path).unwrap().len();
        assert!(size > 0, "{ext} fixture should be non-empty");
    }
}
