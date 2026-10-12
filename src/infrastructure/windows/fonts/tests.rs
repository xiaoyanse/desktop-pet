use super::*;
use std::{
    io::Write,
    time::{SystemTime, UNIX_EPOCH},
};

#[test]
fn mapping_is_read_only_blocks_writes_and_releases_the_file_lock() {
    let path = std::env::temp_dir().join(format!(
        "desktop-meter-mapping-{}-{}.bin",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .unwrap();
    file.write_all(b"read-only font fixture").unwrap();
    drop(file);
    let mapped = Mapping::open(&path).unwrap();
    assert_eq!(mapped.bytes(), b"read-only font fixture");
    assert!(OpenOptions::new().write(true).open(&path).is_err());
    let mut info = MEMORY_BASIC_INFORMATION::default();
    let address = mapped.bytes().as_ptr();
    unsafe {
        assert_ne!(
            VirtualQuery(address.cast(), &mut info, std::mem::size_of_val(&info)),
            0
        );
    }
    assert_eq!(info.Type, MEM_MAPPED);
    assert_eq!(info.Protect, PAGE_READONLY);
    drop(mapped);
    OpenOptions::new()
        .write(true)
        .open(&path)
        .unwrap()
        .write_all(b"R")
        .unwrap();
    assert_eq!(std::fs::read(&path).unwrap()[0], b'R');
    std::fs::remove_file(path).unwrap();
}

#[test]
fn repeated_system_font_requests_borrow_one_process_lifetime_backing() {
    if let Some(first) = load() {
        let second = load().unwrap();
        assert!(matches!(first.bytes, Cow::Borrowed(_)));
        assert_eq!(first.bytes.as_ptr(), second.bytes.as_ptr());
        assert_eq!(first.bytes.len(), second.bytes.len());
        assert!(!first.bytes.is_empty());
    }
}
