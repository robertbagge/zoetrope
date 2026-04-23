use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;

mod common;
use common::{decode_gif, fixture, mov_fixture};

fn zoetrope() -> Command {
    Command::cargo_bin("zoetrope").expect("binary not built")
}

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
fn test_non_mov_extension_errors() {
    let dir = TempDir::new().unwrap();
    // Create an empty .mp4 file (extension check runs before ffmpeg)
    let input = dir.path().join("not-mov.mp4");
    std::fs::write(&input, b"").unwrap();

    zoetrope()
        .arg(&input)
        .assert()
        .failure()
        .stderr(predicate::str::contains("must be a .mov"));
}

#[test]
fn test_png_extension_errors() {
    let dir = TempDir::new().unwrap();
    let input = dir.path().join("image.png");
    std::fs::write(&input, b"").unwrap();

    zoetrope()
        .arg(&input)
        .assert()
        .failure()
        .stderr(predicate::str::contains("must be a .mov"));
}

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

#[test]
fn test_fixture_helper_generates_other_formats() {
    // Verifies the fixture helper works for MP4/WebM/MKV/AVI so Chunk 1 can
    // wire up positive tests without plumbing changes.
    let dir = TempDir::new().unwrap();
    for ext in ["mp4", "webm", "mkv", "avi"] {
        let path = fixture(dir.path(), "probe", ext);
        assert!(path.exists(), "{ext} fixture should exist");
        let size = std::fs::metadata(&path).unwrap().len();
        assert!(size > 0, "{ext} fixture should be non-empty");
    }
}
