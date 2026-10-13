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

pub(super) fn temporary() -> PathBuf {
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

#[test]
fn incremental_package_rebuilds_exact_program_and_rejects_a_changed_baseline() {
    use crate::domain::update::{Delta, Manifest, Release};
    let root = temporary();
    let base = root.join("base");
    program(&base, &vec![7; 15000]);
    let target = root.join("new");
    program(&target, &vec![7; 15000]);
    fs::write(target.join("manifest.json"), br#"{"version":"0.2.1"}"#).unwrap();
    let mut new_program = vec![7; 15000];
    new_program[4000..4020].fill(42);
    fs::write(target.join("desktop-meter.exe"), new_program).unwrap();
    super::delta::create(
        &base.join("desktop-meter.exe"),
        &target.join("desktop-meter.exe"),
        &root.join("patch"),
    )
    .unwrap();
    let package = root.join("delta.zip");
    let mut writer = zip::ZipWriter::new(fs::File::create(&package).unwrap());
    for name in [
        "desktop-meter.exe.patch",
        "README.md",
        "启动设置.cmd",
        "manifest.json",
    ] {
        writer
            .start_file(
                format!("DesktopMeter-0.2.1-from-0.2.0-delta-win-x64/{name}"),
                zip::write::SimpleFileOptions::default(),
            )
            .unwrap();
        writer
            .write_all(
                &fs::read(if name.ends_with(".patch") {
                    root.join("patch")
                } else {
                    target.join(name)
                })
                .unwrap(),
            )
            .unwrap();
    }
    writer.finish().unwrap();
    let release = Release {
        version: "0.2.1".into(),
        notes: String::new(),
        url: String::new(),
        page: String::new(),
        source: http::GITHUB.into(),
        manifest: Manifest {
            schema: 1,
            version: "0.2.1".into(),
            file_name: "DesktopMeter-0.2.1-win-x64.zip".into(),
            size: 100000,
            sha256: "a".repeat(64),
            executable_sha256: files::hash(&target.join("desktop-meter.exe")).unwrap(),
            delta: Some(Delta {
                base_version: "0.2.0".into(),
                base_executable_sha256: files::hash(&base.join("desktop-meter.exe")).unwrap(),
                executable_size: 15000,
                file_name: "DesktopMeter-0.2.1-from-0.2.0-delta-win-x64.zip".into(),
                size: fs::metadata(&package).unwrap().len(),
                sha256: files::hash(&package).unwrap(),
                url: "https://github.com/xiaoyanse/desktop-pet/releases/download/v0.2.1/delta.zip"
                    .into(),
            }),
        },
    };
    super::delta_package::extract(
        &package,
        &root.join("payload"),
        &base.join("desktop-meter.exe"),
        &release,
        &std::sync::atomic::AtomicBool::new(false),
    )
    .unwrap();
    assert_eq!(
        files::hash(&root.join("payload/desktop-meter.exe")).unwrap(),
        release.manifest.executable_sha256
    );
    fs::write(base.join("desktop-meter.exe"), b"modified").unwrap();
    assert!(!super::delta_package::matches(
        release.manifest.delta.as_ref().unwrap(),
        &release.version,
        &base.join("desktop-meter.exe")
    ));
    assert!(
        super::delta_package::extract(
            &package,
            &root.join("bad"),
            &base.join("desktop-meter.exe"),
            &release,
            &std::sync::atomic::AtomicBool::new(false)
        )
        .is_err()
    );
    assert!(!root.join("bad").exists());
    fs::remove_dir_all(root).unwrap();
}
