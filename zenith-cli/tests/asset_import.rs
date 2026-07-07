use std::fs;
use std::process::Command;

fn minimal_doc() -> &'static str {
    r#"zenith version=1 {
  project id="proj.asset" name="Asset Test"
  tokens format="zenith-token-v1" { }
  styles { }
  assets { }
  document id="doc.asset" title="Asset Test" {
    page id="page.main" w=(px)100 h=(px)100 { }
  }
}"#
}

fn asset_import_command<'a>(input: &'a std::path::Path, doc: &'a std::path::Path) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_zenith"));
    command
        .arg("asset")
        .arg("import")
        .arg(input)
        .arg("--into")
        .arg(doc)
        .arg("--id")
        .arg("asset.logo")
        .arg("--src")
        .arg("assets/logo.svg")
        .arg("--kind")
        .arg("svg");
    command
}

#[test]
fn asset_import_dry_run_does_not_write_asset_or_doc() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let doc = tmp.path().join("poster.zen");
    let input = tmp.path().join("logo.svg");
    fs::write(&doc, minimal_doc()).expect("write doc");
    fs::write(&input, "<svg/>").expect("write input");

    let output = asset_import_command(&input, &doc)
        .output()
        .expect("run zenith");

    assert!(output.status.success());
    assert_eq!(fs::read_to_string(&doc).expect("read doc"), minimal_doc());
    assert!(!tmp.path().join("assets/logo.svg").exists());
}

#[test]
fn asset_import_apply_writes_asset_and_doc() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let doc = tmp.path().join("poster.zen");
    let input = tmp.path().join("logo.svg");
    fs::write(&doc, minimal_doc()).expect("write doc");
    fs::write(&input, "<svg/>").expect("write input");

    let output = asset_import_command(&input, &doc)
        .arg("--apply")
        .output()
        .expect("run zenith");

    assert!(output.status.success());
    assert_eq!(
        fs::read(tmp.path().join("assets/logo.svg")).expect("read asset"),
        b"<svg/>"
    );
    let updated = fs::read_to_string(&doc).expect("read doc");
    assert!(updated.contains(r#"id="asset.logo""#));
    assert!(updated.contains(r#"src="assets/logo.svg""#));
    assert!(updated.contains("sha256="));
}

#[test]
fn asset_import_apply_refuses_different_existing_destination() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let doc = tmp.path().join("poster.zen");
    let input = tmp.path().join("logo.svg");
    let dest = tmp.path().join("assets/logo.svg");
    fs::create_dir_all(dest.parent().expect("dest parent")).expect("mkdir");
    fs::write(&doc, minimal_doc()).expect("write doc");
    fs::write(&input, "<svg/>").expect("write input");
    fs::write(&dest, "different").expect("write existing");

    let output = asset_import_command(&input, &doc)
        .arg("--apply")
        .output()
        .expect("run zenith");

    assert_eq!(output.status.code(), Some(2));
    assert_eq!(fs::read_to_string(&doc).expect("read doc"), minimal_doc());
    assert_eq!(fs::read_to_string(&dest).expect("read dest"), "different");
}
