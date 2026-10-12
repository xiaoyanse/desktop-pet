use super::*;
use crate::domain::office::{Command, Kind};
fn directory() -> PathBuf {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("evidence")
        .join(format!(
            "office-test-{}-{}",
            std::process::id(),
            SERIAL.fetch_add(1, Ordering::Relaxed)
        ));
    fs::create_dir_all(&root).unwrap();
    root
}
fn sample(title: &str) -> Book {
    Book::default()
        .changed(
            Command::Save {
                id: None,
                kind: Kind::Note,
                title: title.into(),
                body: "中文\n本地文字".into(),
                pinned: true,
                due: None,
            },
            1,
        )
        .unwrap()
}
#[test]
fn atomic_file_roundtrip_retains_previous_backup_and_bad_file_is_not_overwritten() {
    let root = directory();
    let store = LocalOfficeStore::at(root.join("workspace.json"));
    assert_eq!(store.load().unwrap(), Book::default());
    assert!(!store.path.exists());
    let first = sample("第一版");
    store.save(&first, &Book::default()).unwrap();
    let second = sample("第二版");
    store.save(&second, &first).unwrap();
    assert_eq!(store.load().unwrap(), second);
    let backup: Book =
        serde_json::from_slice(&fs::read(root.join("workspace.backup.json")).unwrap()).unwrap();
    assert_eq!(backup, first);
    fs::write(&store.path, b"corrupt").unwrap();
    assert!(store.load().is_err());
    assert!(store.save(&second, &second).is_err());
    assert_eq!(fs::read(&store.path).unwrap(), b"corrupt");
    assert_eq!(fs::read_dir(&root).unwrap().count(), 2);
    for name in ["workspace.json", "workspace.backup.json"] {
        fs::remove_file(root.join(name)).unwrap();
    }
    fs::remove_dir(root).unwrap();
}
#[test]
fn future_version_and_oversized_file_are_refused_without_edits() {
    let root = directory();
    let store = LocalOfficeStore::at(root.join("workspace.json"));
    let mut future = sample("未来数据");
    future.version = 2;
    fs::write(&store.path, serde_json::to_vec(&future).unwrap()).unwrap();
    assert!(store.load().is_err());
    let file = File::create(&store.path).unwrap();
    file.set_len(MAX_FILE_BYTES + 1).unwrap();
    drop(file);
    assert!(store.load().is_err());
    assert_eq!(fs::metadata(&store.path).unwrap().len(), MAX_FILE_BYTES + 1);
    fs::remove_file(&store.path).unwrap();
    fs::remove_dir(root).unwrap();
}

#[test]
fn external_valid_change_blocks_stale_save_without_overwriting_either_file() {
    let root = directory();
    let store = LocalOfficeStore::at(root.join("workspace.json"));
    let first = sample("原始内容");
    store.save(&first, &Book::default()).unwrap();
    let external = sample("外部新内容");
    let raw = serde_json::to_vec_pretty(&external).unwrap();
    fs::write(&store.path, &raw).unwrap();
    assert!(store.save(&sample("旧编辑"), &first).is_err());
    assert_eq!(fs::read(&store.path).unwrap(), raw);
    assert_eq!(fs::read_dir(&root).unwrap().count(), 1);
    fs::remove_file(&store.path).unwrap();
    fs::remove_dir(root).unwrap();
}
