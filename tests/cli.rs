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

// ─── --max-size ─────────────────────────────────────────────────────────────

#[test]
fn test_max_size_respects_limit() {
    let dir = TempDir::new().unwrap();
    let input = mov_fixture(dir.path());
    let output = dir.path().join("in.gif");

    // The 2s testsrc fixture at default medium settings is comfortably
    // over 80 KB; asking for 80 KB forces the fit loop to shrink.
    zoetrope()
        .arg(&input)
        .args(["--max-size", "80kb"])
        .assert()
        .success();

    let size = std::fs::metadata(&output).unwrap().len();
    assert!(
        size <= 80_000,
        "output should be ≤ 80_000 bytes, got {size}"
    );
}

#[test]
fn test_max_size_impossible_target_errors() {
    let dir = TempDir::new().unwrap();
    let input = mov_fixture(dir.path());

    // 1 KB is below any plausible GIF floor — fit loop exhausts and errors.
    zoetrope()
        .arg(&input)
        .args(["--max-size", "1kb"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("could not reach"));
}

#[test]
fn test_max_size_rejects_bad_unit() {
    let dir = TempDir::new().unwrap();
    let input = mov_fixture(dir.path());

    zoetrope()
        .arg(&input)
        .args(["--max-size", "5xb"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("invalid size"));
}

#[test]
fn test_max_size_accepts_raw_bytes() {
    let dir = TempDir::new().unwrap();
    let input = mov_fixture(dir.path());
    let output = dir.path().join("in.gif");

    // 2_000_000 bytes = 2 MB. Default medium gif should fit without shrinking.
    zoetrope()
        .arg(&input)
        .args(["--max-size", "2000000"])
        .assert()
        .success();

    let size = std::fs::metadata(&output).unwrap().len();
    assert!(size <= 2_000_000);
}

// ─── --for <PLATFORM> ───────────────────────────────────────────────────────

#[test]
fn test_for_slack_produces_gif_under_5mb_at_480px() {
    assert_platform_produces_gif("slack", 5_000_000, 480);
}

#[test]
fn test_for_email_produces_gif_under_500kb_at_320px() {
    assert_platform_produces_gif("email", 500_000, 320);
}

#[test]
fn test_for_github_under_10mb_and_960px() {
    assert_platform_produces_gif("github", 10_000_000, 960);
}

#[test]
fn test_for_discord_under_8mb_and_640px() {
    assert_platform_produces_gif("discord", 8_000_000, 640);
}

#[test]
fn test_for_twitter_under_5mb_and_480px() {
    assert_platform_produces_gif("twitter", 5_000_000, 480);
}

fn assert_platform_produces_gif(platform: &str, max_bytes: u64, expected_width: u16) {
    let dir = TempDir::new().unwrap();
    let input = mov_fixture(dir.path());
    let output = dir.path().join("in.gif");

    zoetrope()
        .arg(&input)
        .args(["--for", platform])
        .assert()
        .success();

    let size = std::fs::metadata(&output).unwrap().len();
    assert!(
        size <= max_bytes,
        "--for {platform} output should be ≤ {max_bytes} bytes, got {size}"
    );

    let (w, _, _) = decode_gif(&output);
    assert_eq!(w, expected_width, "--for {platform} width");
}

#[test]
fn test_for_with_webp_errors() {
    let dir = TempDir::new().unwrap();
    let input = mov_fixture(dir.path());

    zoetrope()
        .arg(&input)
        .args(["--for", "slack", "-F", "webp"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("produces GIF"));
}

#[test]
fn test_for_with_webp_output_path_errors() {
    let dir = TempDir::new().unwrap();
    let input = mov_fixture(dir.path());
    let output = dir.path().join("out.webp");

    zoetrope()
        .arg(&input)
        .args(["--for", "slack", "-o"])
        .arg(&output)
        .assert()
        .failure()
        .stderr(predicate::str::contains("produces GIF"));
}

#[test]
fn test_for_with_explicit_fps_uses_explicit_fps() {
    let dir = TempDir::new().unwrap();
    let input = mov_fixture(dir.path());
    let output = dir.path().join("in.gif");

    // --for slack would default fps=10, but --fps 15 must win.
    zoetrope()
        .arg(&input)
        .args(["--for", "slack", "--fps", "15"])
        .assert()
        .success();

    let (_, _, frames) = decode_gif(&output);
    // 2s fixture @ 15fps ≈ 30 frames (with a tolerance for gifski de-duping).
    assert!(
        (25..=35).contains(&frames),
        "expected ~30 frames (2s @ 15fps), got {frames}"
    );
}

#[test]
fn test_for_with_explicit_quality_uses_quality_preset() {
    let dir = TempDir::new().unwrap();
    let input = mov_fixture(dir.path());
    let output = dir.path().join("in.gif");

    // --for slack defaults to 480px; -q ultra must win and produce 2048px.
    // Disable the slack size cap (raise to something unreachable) so the
    // fit loop doesn't shrink us away from ultra's 2048px.
    zoetrope()
        .arg(&input)
        .args(["--for", "slack", "-q", "ultra", "--max-size", "1gb"])
        .assert()
        .success();

    let (w, _, _) = decode_gif(&output);
    assert_eq!(w, 2048, "-q ultra should override --for slack's 480px");
}

#[test]
fn test_for_with_explicit_max_size_uses_user_target() {
    let dir = TempDir::new().unwrap();
    let input = mov_fixture(dir.path());
    let output = dir.path().join("in.gif");

    // Slack defaults to 5MB; user's 1MB must override.
    zoetrope()
        .arg(&input)
        .args(["--for", "slack", "--max-size", "1mb"])
        .assert()
        .success();

    let size = std::fs::metadata(&output).unwrap().len();
    assert!(
        size <= 1_000_000,
        "user --max-size should override slack preset, got {size}"
    );
}

#[test]
fn test_unknown_platform_errors() {
    let dir = TempDir::new().unwrap();
    let input = mov_fixture(dir.path());

    zoetrope()
        .arg(&input)
        .args(["--for", "myspace"])
        .assert()
        .failure();
}

// ─── Batch conversion ──────────────────────────────────────────────────────

#[test]
fn test_batch_multiple_inputs() {
    let dir = TempDir::new().unwrap();
    let a = fixture(dir.path(), "a", "mov");
    let b = fixture(dir.path(), "b", "mp4");

    zoetrope().arg(&a).arg(&b).assert().success();

    let out_a = dir.path().join("a.gif");
    let out_b = dir.path().join("b.gif");
    assert!(out_a.exists(), "batch output a.gif should exist");
    assert!(out_b.exists(), "batch output b.gif should exist");
    assert!(std::fs::metadata(&out_a).unwrap().len() > 0);
    assert!(std::fs::metadata(&out_b).unwrap().len() > 0);
}

#[test]
fn test_batch_output_dir() {
    let dir = TempDir::new().unwrap();
    let out_dir = dir.path().join("gifs");
    let a = fixture(dir.path(), "a", "mov");
    let b = fixture(dir.path(), "b", "mov");

    zoetrope()
        .arg(&a)
        .arg(&b)
        .args(["--output-dir".as_ref(), out_dir.as_os_str()])
        .assert()
        .success();

    assert!(
        out_dir.join("a.gif").exists(),
        "a.gif should land in --output-dir"
    );
    assert!(
        out_dir.join("b.gif").exists(),
        "b.gif should land in --output-dir"
    );
    assert!(
        !dir.path().join("a.gif").exists(),
        "a.gif should NOT land next to input when --output-dir is set"
    );
}

#[test]
fn test_batch_output_dir_creates_missing() {
    let dir = TempDir::new().unwrap();
    let out_dir = dir.path().join("nested/newly/created");
    let a = mov_fixture(dir.path());

    zoetrope()
        .arg(&a)
        .args(["--output-dir".as_ref(), out_dir.as_os_str()])
        .assert()
        .success();

    assert!(out_dir.exists(), "missing output dir should be created");
    assert!(out_dir.join("in.gif").exists());
}

#[test]
fn test_batch_with_o_flag_errors() {
    let dir = TempDir::new().unwrap();
    let a = fixture(dir.path(), "a", "mov");
    let b = fixture(dir.path(), "b", "mov");
    let custom = dir.path().join("combined.gif");

    zoetrope()
        .arg(&a)
        .arg(&b)
        .args(["-o".as_ref(), custom.as_os_str()])
        .assert()
        .failure()
        .stderr(predicate::str::contains("--output-dir"));
}

#[test]
fn test_batch_applies_flags_uniformly() {
    let dir = TempDir::new().unwrap();
    let a = fixture(dir.path(), "a", "mov");
    let b = fixture(dir.path(), "b", "mov");

    zoetrope()
        .arg(&a)
        .arg(&b)
        .args(["--width", "400"])
        .assert()
        .success();

    let (wa, _, _) = decode_gif(&dir.path().join("a.gif"));
    let (wb, _, _) = decode_gif(&dir.path().join("b.gif"));
    assert_eq!(wa, 400, "a.gif should honor --width");
    assert_eq!(wb, 400, "b.gif should honor --width");
}

#[test]
fn test_batch_preflight_aborts_on_missing_input() {
    // If one of the inputs doesn't exist, the batch aborts during planning —
    // no partial-encode side effects. Continue-on-error covers *runtime*
    // encode failures (per-file), not bad configs.
    let dir = TempDir::new().unwrap();
    let a = fixture(dir.path(), "a", "mov");
    let missing = dir.path().join("missing.mov");
    let b = fixture(dir.path(), "b", "mov");

    zoetrope()
        .arg(&a)
        .arg(&missing)
        .arg(&b)
        .assert()
        .failure()
        .stderr(predicate::str::contains("file not found"));

    assert!(
        !dir.path().join("a.gif").exists(),
        "pre-flight failure should abort before any encode"
    );
    assert!(!dir.path().join("b.gif").exists());
}

#[test]
fn test_single_input_with_output_dir() {
    let dir = TempDir::new().unwrap();
    let out_dir = dir.path().join("out");
    let a = mov_fixture(dir.path());

    zoetrope()
        .arg(&a)
        .args(["--output-dir".as_ref(), out_dir.as_os_str()])
        .assert()
        .success();

    assert!(
        out_dir.join("in.gif").exists(),
        "single-input + --output-dir"
    );
    assert!(
        !dir.path().join("in.gif").exists(),
        "default path should not be used when --output-dir is given"
    );
}

#[test]
fn test_output_dir_and_output_conflict() {
    let dir = TempDir::new().unwrap();
    let a = mov_fixture(dir.path());
    let out_dir = dir.path().join("out");
    let custom = dir.path().join("custom.gif");

    zoetrope()
        .arg(&a)
        .args(["--output-dir".as_ref(), out_dir.as_os_str()])
        .args(["-o".as_ref(), custom.as_os_str()])
        .assert()
        .failure();
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
