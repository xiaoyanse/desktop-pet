#[cfg(test)]
mod tests;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

pub const MAX_ITEMS: usize = 512;
pub const MAX_TEXT_BYTES: usize = 2 * 1024 * 1024;
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Kind {
    Note,
    Task,
}
impl Kind {
    pub fn label(self) -> &'static str {
        match self {
            Self::Note => "便签",
            Self::Task => "待办",
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Item {
    pub id: u64,
    pub kind: Kind,
    pub title: String,
    pub body: String,
    pub pinned: bool,
    pub done: bool,
    pub due: Option<u64>,
    pub acknowledged_due: Option<u64>,
    pub updated: u64,
}
impl Item {
    pub fn needs_reminder(&self, now: u64) -> bool {
        self.kind == Kind::Task
            && !self.done
            && self
                .due
                .is_some_and(|due| due <= now && self.acknowledged_due != Some(due))
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Book {
    pub version: u32,
    pub next_id: u64,
    pub items: Vec<Item>,
}
impl Default for Book {
    fn default() -> Self {
        Self {
            version: 1,
            next_id: 1,
            items: vec![],
        }
    }
}
pub enum Command {
    Save {
        id: Option<u64>,
        kind: Kind,
        title: String,
        body: String,
        pinned: bool,
        due: Option<u64>,
    },
    Complete(u64),
    Pin(u64),
    Delete(u64),
    Dismiss(u64),
    Snooze(u64, u64),
}
impl Book {
    pub fn validate(&self) -> Result<(), String> {
        if self.version != 1 {
            return Err("便签文件版本不受支持，原文件未修改".into());
        }
        if self.items.len() > MAX_ITEMS {
            return Err("便签和待办合计最多512项".into());
        }
        let mut ids = HashSet::new();
        let mut bytes = 0;
        for item in &self.items {
            if item.id == 0 || item.id >= self.next_id || !ids.insert(item.id) {
                return Err("便签编号异常，原文件未修改".into());
            }
            if item.title.trim().is_empty() || item.title.len() > 256 || item.body.len() > 32768 {
                return Err("标题需1–256字节，正文最多32 KiB".into());
            }
            if item.kind == Kind::Note
                && (item.done || item.due.is_some() || item.acknowledged_due.is_some())
            {
                return Err("便签不能包含待办状态".into());
            }
            if item.acknowledged_due.is_some() && item.acknowledged_due != item.due {
                return Err("提醒状态与到期时间不一致".into());
            }
            if item.due.is_some_and(|due| due > 253402300799) {
                return Err("提醒时间超过支持范围".into());
            }
            bytes += item.title.len() + item.body.len();
        }
        if self.next_id == 0 || bytes > MAX_TEXT_BYTES {
            return Err("便签文本总量最多2 MiB".into());
        }
        Ok(())
    }
    pub fn changed(&self, command: Command, now: u64) -> Result<Self, String> {
        let mut next = self.clone();
        match command {
            Command::Save {
                id,
                kind,
                title,
                body,
                pinned,
                due,
            } => {
                let (id, index) = match id {
                    Some(id) => (
                        id,
                        Some(
                            next.items
                                .iter()
                                .position(|i| i.id == id)
                                .ok_or("该条目已不存在")?,
                        ),
                    ),
                    None => {
                        let id = next.next_id;
                        next.next_id = id.checked_add(1).ok_or("条目编号已达上限")?;
                        (id, None)
                    }
                };
                let old = index.map(|i| &next.items[i]);
                let due = if kind == Kind::Task { due } else { None };
                let item = Item {
                    id,
                    kind,
                    title: title.trim().to_string(),
                    body,
                    pinned,
                    done: kind == Kind::Task && old.is_some_and(|i| i.done),
                    due,
                    acknowledged_due: old
                        .filter(|i| i.due == due)
                        .and_then(|i| i.acknowledged_due),
                    updated: now,
                };
                if let Some(index) = index {
                    next.items[index] = item;
                } else {
                    next.items.push(item);
                }
            }
            Command::Delete(id) => {
                let index = next
                    .items
                    .iter()
                    .position(|i| i.id == id)
                    .ok_or("该条目已不存在")?;
                next.items.remove(index);
            }
            command => {
                let id = match command {
                    Command::Complete(id)
                    | Command::Pin(id)
                    | Command::Dismiss(id)
                    | Command::Snooze(id, _) => id,
                    _ => unreachable!(),
                };
                let item = next
                    .items
                    .iter_mut()
                    .find(|i| i.id == id)
                    .ok_or("该条目已不存在")?;
                match command {
                    Command::Pin(_) => item.pinned = !item.pinned,
                    Command::Complete(_) if item.kind == Kind::Task => {
                        item.done = !item.done;
                        if !item.done {
                            item.acknowledged_due = None;
                        }
                    }
                    Command::Dismiss(_) if item.kind == Kind::Task => {
                        item.acknowledged_due = item.due
                    }
                    Command::Snooze(_, seconds)
                        if item.kind == Kind::Task
                            && !item.done
                            && (60..=86400).contains(&seconds) =>
                    {
                        item.due = Some(now.checked_add(seconds).ok_or("提醒时间超出范围")?);
                        item.acknowledged_due = None;
                    }
                    _ => return Err("此操作不适用于该条目".into()),
                }
                item.updated = now;
            }
        }
        next.validate()?;
        Ok(next)
    }
}
