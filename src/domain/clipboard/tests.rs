use super::*;
fn text(value: &str) -> Packet {
    Packet::text(value.into()).unwrap()
}
#[test]
fn budget_eviction_preserves_pins_and_refused_insert_is_atomic() {
    let limits = ClipboardConfig { count: 2, mib: 4 };
    let mut h = History::default();
    h.insert(text("中文"), 1, &limits).unwrap();
    h.pin(1);
    h.insert(text("second"), 2, &limits).unwrap();
    h.insert(text("third"), 3, &limits).unwrap();
    assert_eq!(h.items.len(), 2);
    assert!(h.items.iter().any(|i| i.id == 1));
    assert!(!h.items.iter().any(|i| i.id == 2));
    h.pin(3);
    assert!(h.insert(text("fourth"), 4, &limits).is_err());
    assert_eq!(h.items.len(), 2);
    assert!(h.trim(&ClipboardConfig { count: 1, mib: 4 }).is_err());
    assert_eq!(h.items.len(), 2);
}
#[test]
fn duplicates_preserve_identity_and_pins_while_full_image_is_retained() {
    let mut h = History::default();
    let limits = ClipboardConfig::default();
    h.insert(text("line\n中"), 1, &limits).unwrap();
    h.pin(1);
    h.insert(text("line\n中"), 3, &limits).unwrap();
    assert_eq!(h.items.len(), 1);
    assert_eq!(h.items[0].id, 1);
    assert!(h.items[0].pinned);
    assert_eq!(h.items[0].captured, 3);
    let image = Image::new([400, 200], vec![125; 400 * 200 * 4]).unwrap();
    let p = Packet::picture(image).unwrap();
    assert_eq!(p.thumbnail.as_ref().unwrap().size(), [160, 80]);
    assert_eq!(p.bytes, 400 * 200 * 4 + 160 * 80 * 4);
    assert!(matches!(&p.content,Content::Picture(i) if i.size()==[400,200]));
    h.insert(p, 4, &limits).unwrap();
    h.clear();
    assert!(h.items.is_empty());
}
#[test]
fn malformed_or_oversized_text_and_configuration_are_bounded() {
    assert!(Packet::text(String::new()).is_err());
    assert!(Packet::text("x".repeat(MAX_TEXT + 1)).is_err());
    let mut c = ClipboardConfig {
        count: usize::MAX,
        mib: usize::MAX,
    };
    c.normalize();
    assert_eq!(c.count, 200);
    assert_eq!(c.mib, 64);
}
#[test]
fn identity_exhaustion_does_not_evict_existing_content_and_revisions_change() {
    let mut h = History::default();
    let limits = ClipboardConfig { count: 1, mib: 4 };
    h.insert(text("保留"), 1, &limits).unwrap();
    let revision = h.revision();
    h.next = u64::MAX;
    assert!(h.insert(text("不能加入"), 2, &limits).is_err());
    assert_eq!(h.items.len(), 1);
    assert_eq!(h.items[0].packet.label(), "保留");
    assert_eq!(h.revision(), revision);
    h.pin(1);
    assert_ne!(h.revision(), revision);
    h.clear();
    assert!(h.items.is_empty());
}
