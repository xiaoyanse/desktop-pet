use super::*;
fn save(kind: Kind, due: Option<u64>) -> Command {
    Command::Save {
        id: None,
        kind,
        title: "写需求".into(),
        body: "中文工作线索与命令".into(),
        pinned: false,
        due,
    }
}
#[test]
fn crud_roundtrip_preserves_chinese_and_note_has_no_task_state() {
    let book = Book::default()
        .changed(save(Kind::Note, Some(10)), 1)
        .unwrap();
    assert!(book.items[0].due.is_none());
    assert_eq!(book.items[0].title, "写需求");
    let book = book.changed(Command::Pin(1), 2).unwrap();
    assert!(book.items[0].pinned);
    assert!(book.changed(Command::Complete(1), 3).is_err());
    let encoded = serde_json::to_vec(&book).unwrap();
    assert_eq!(serde_json::from_slice::<Book>(&encoded).unwrap(), book);
    let empty = book.changed(Command::Delete(1), 4).unwrap();
    assert!(empty.items.is_empty());
    assert_eq!(empty.next_id, 2);
}
#[test]
fn reminders_follow_completion_snooze_and_changed_deadline() {
    let book = Book::default()
        .changed(save(Kind::Task, Some(20)), 1)
        .unwrap();
    assert!(!book.items[0].needs_reminder(19));
    assert!(book.items[0].needs_reminder(20));
    let dismissed = book.changed(Command::Dismiss(1), 20).unwrap();
    assert!(!dismissed.items[0].needs_reminder(300));
    let snoozed = dismissed.changed(Command::Snooze(1, 600), 300).unwrap();
    assert_eq!(snoozed.items[0].due, Some(900));
    assert!(snoozed.items[0].needs_reminder(900));
    let done = snoozed.changed(Command::Complete(1), 900).unwrap();
    assert!(!done.items[0].needs_reminder(999));
    let restored = done.changed(Command::Complete(1), 999).unwrap();
    assert!(restored.items[0].needs_reminder(999));
    let changed = dismissed
        .changed(
            Command::Save {
                id: Some(1),
                kind: Kind::Task,
                title: "新期限".into(),
                body: String::new(),
                pinned: false,
                due: Some(30),
            },
            21,
        )
        .unwrap();
    assert!(changed.items[0].needs_reminder(30));
}
#[test]
fn rejects_bad_versions_ids_and_capacity_without_changing_original() {
    let book = Book::default();
    assert!(
        book.changed(
            Command::Save {
                id: None,
                kind: Kind::Note,
                title: " ".into(),
                body: String::new(),
                pinned: false,
                due: None
            },
            1
        )
        .is_err()
    );
    assert!(
        book.changed(
            Command::Save {
                id: None,
                kind: Kind::Note,
                title: "a".into(),
                body: "a".repeat(32769),
                pinned: false,
                due: None
            },
            1
        )
        .is_err()
    );
    let mut bad = book.changed(save(Kind::Task, Some(1)), 1).unwrap();
    bad.items.push(bad.items[0].clone());
    assert!(bad.validate().is_err());
    bad.items.pop();
    bad.version = 2;
    assert!(bad.validate().is_err());
    bad.version = 1;
    bad.items = vec![bad.items[0].clone(); 513];
    assert!(bad.validate().is_err());
    assert!(book.items.is_empty());
    assert_eq!(book.next_id, 1);
}
