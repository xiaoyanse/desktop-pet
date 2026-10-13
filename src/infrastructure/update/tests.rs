use super::{
    files::{self, Job},
    http, install,
};
use std::{
    fs,
    io::Write,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

fn temporary() -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "desktop-meter-update-test-{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir(&path).unwrap();
    path
}
fn program(path: &std::path::Path, bytes: &[u8]) {
    fs::create_dir(path).unwrap();
    for name in files::FILES {
        fs::write(
            path.join(name),
            if name == "manifest.json" {
                br#"{"version":"0.2.0"}"#.as_slice()
            } else {
                bytes
            },
        )
        .unwrap();
    }
}
#[test]
fn download_urls_reject_external_hosts_plain_http_other_repositories_and_credentials() {
    assert!(
        http::asset_url(
            "https://github.com/xiaoyanse/desktop-pet/releases/download/v0.2.0/package.zip",
            http::GITHUB
        )
        .is_ok()
    );
    for url in [
        "http://github.com/xiaoyanse/desktop-pet/releases/download/v0.2.0/package.zip",
        "https://evil.example/xiaoyanse/desktop-pet/releases/package.zip",
        "https://github.com/another/repo/releases/package.zip",
        "https://user:password@github.com/xiaoyanse/desktop-pet/releases/package.zip",
        "https://github.com/xiaoyanse/desktop-pet/releases/%2f..%2ffile",
        "https://github.com/xiaoyanse/desktop-pet/releases/package.zip?token=secret",
    ] {
        assert!(http::asset_url(url, http::GITHUB).is_err(), "{url}");
    }
}
#[test]
fn replacement_refuses_extra_user_files_and_keeps_backup_on_success() {
    let root = temporary();
    let target = root.join("application");
    program(&target, b"old");
    let payload = root.join("payload");
    program(&payload, b"new");
    let job = Job {
        target: target.clone(),
        old_hash: files::hash(&target.join("desktop-meter.exe")).unwrap(),
        version: "0.2.0".into(),
        new_hash: files::hash(&payload.join("desktop-meter.exe")).unwrap(),
        nonce: "1234".into(),
        parent: 1,
    };
    fs::write(target.join("user-notes.txt"), "keep me").unwrap();
    assert!(install::replace(&root, &job).is_err());
    assert_eq!(fs::read(target.join("desktop-meter.exe")).unwrap(), b"old");
    fs::remove_file(target.join("user-notes.txt")).unwrap();
    let backup = install::replace(&root, &job).unwrap();
    assert_eq!(fs::read(target.join("desktop-meter.exe")).unwrap(), b"new");
    assert_eq!(fs::read(backup.join("desktop-meter.exe")).unwrap(), b"old");
    fs::remove_dir_all(root).unwrap();
}
#[test]
fn changed_original_hash_blocks_install_before_any_directory_moves() {
    let root = temporary();
    let target = root.join("application");
    program(&target, b"old");
    program(&root.join("payload"), b"new");
    let job = Job {
        target: target.clone(),
        old_hash: "0".repeat(64),
        version: "0.2.0".into(),
        new_hash: files::hash(&root.join("payload/desktop-meter.exe")).unwrap(),
        nonce: "1234".into(),
        parent: 1,
    };
    assert!(install::replace(&root, &job).is_err());
    assert!(target.is_dir());
    assert!(root.join("payload").is_dir());
    fs::remove_dir_all(root).unwrap();
}
fn make_zip(path: &std::path::Path, names: &[&str]) {
    let mut writer = zip::ZipWriter::new(fs::File::create(path).unwrap());
    for name in names {
        writer
            .start_file(*name, zip::write::SimpleFileOptions::default())
            .unwrap();
        writer.write_all(b"fake").unwrap();
    }
    writer.finish().unwrap();
}
#[test]
fn malicious_archive_cannot_escape_or_inject_extra_files() {
    for name in [
        "../outside",
        "DesktopMeter-0.2.0-win-x64/../outside",
        "DesktopMeter-0.2.0-win-x64/evil.dll",
        "DesktopMeter-0.2.0-win-x64/desktop-meter.exe/evil",
    ] {
        let root = temporary();
        let zip = root.join("test.zip");
        make_zip(&zip, &[name]);
        assert!(files::extract(&zip, &root.join("payload"), "0.2.0", &"a".repeat(64)).is_err());
        assert!(!root.join("outside").exists());
        fs::remove_dir_all(root).unwrap();
    }
}
