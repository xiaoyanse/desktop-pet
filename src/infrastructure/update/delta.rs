use std::{
    collections::HashMap,
    fs::{self, File},
    io::{Read, Seek, SeekFrom, Write},
    path::Path,
    sync::atomic::{AtomicBool, Ordering},
};

const BLOCK: usize = 4096;
const LIMIT: u64 = 100 * 1024 * 1024;
const MODULUS: u32 = 65521;
const MAGIC: &[u8; 8] = b"DMDELTA1";

fn checksum(bytes: &[u8]) -> (u32, u32) {
    let first = bytes.iter().map(|b| *b as u32).sum::<u32>() % MODULUS;
    let second = bytes
        .iter()
        .enumerate()
        .map(|(i, b)| (bytes.len() - i) as u32 * *b as u32)
        .sum::<u32>()
        % MODULUS;
    (first, second)
}
fn key(sum: (u32, u32)) -> u32 {
    sum.0 | (sum.1 << 16)
}
fn literal(writer: &mut File, bytes: &[u8]) -> std::io::Result<()> {
    for chunk in bytes.chunks(65536) {
        writer.write_all(&[0])?;
        writer.write_all(&(chunk.len() as u32).to_le_bytes())?;
        writer.write_all(chunk)?;
    }
    Ok(())
}
pub(super) fn create(base: &Path, new: &Path, patch: &Path) -> Result<(), String> {
    if [base, new]
        .iter()
        .any(|path| fs::metadata(path).map_or(true, |m| !(1..=LIMIT).contains(&m.len())))
    {
        return Err("差分源文件大小无效".into());
    }
    let source = fs::read(base).map_err(|_| "差分源文件不可读")?;
    let target = fs::read(new).map_err(|_| "差分目标文件不可读")?;
    let mut blocks: HashMap<u32, Vec<usize>> = HashMap::new();
    for offset in (0..source.len().saturating_sub(BLOCK - 1)).step_by(BLOCK) {
        blocks
            .entry(key(checksum(&source[offset..offset + BLOCK])))
            .or_default()
            .push(offset);
    }
    let mut writer = File::options()
        .write(true)
        .create_new(true)
        .open(patch)
        .map_err(|_| "差分输出已存在或不可写")?;
    let result = (|| -> std::io::Result<()> {
        writer.write_all(MAGIC)?;
        writer.write_all(&(target.len() as u64).to_le_bytes())?;
        let mut position = 0;
        let mut start = 0;
        let mut sum = if target.len() >= BLOCK {
            checksum(&target[..BLOCK])
        } else {
            (0, 0)
        };
        while position + BLOCK <= target.len() {
            let found = blocks.get(&key(sum)).and_then(|offsets| {
                offsets.iter().take(64).copied().find(|offset| {
                    source[*offset..*offset + BLOCK] == target[position..position + BLOCK]
                })
            });
            if let Some(offset) = found {
                literal(&mut writer, &target[start..position])?;
                let mut length = BLOCK;
                while offset + length < source.len()
                    && position + length < target.len()
                    && source[offset + length] == target[position + length]
                {
                    length += 1;
                }
                writer.write_all(&[1])?;
                writer.write_all(&(offset as u64).to_le_bytes())?;
                writer.write_all(&(length as u32).to_le_bytes())?;
                position += length;
                start = position;
                if position + BLOCK <= target.len() {
                    sum = checksum(&target[position..position + BLOCK]);
                }
            } else {
                if position + BLOCK < target.len() {
                    let old = target[position] as u32;
                    sum.0 = (sum.0 + MODULUS - old + target[position + BLOCK] as u32) % MODULUS;
                    sum.1 = (sum.1 + MODULUS * 17 - (BLOCK as u32 * old) + sum.0) % MODULUS;
                }
                position += 1;
            }
        }
        literal(&mut writer, &target[start..])?;
        writer.sync_all()
    })();
    result.map_err(|_| "生成差分文件失败".into())
}
fn number<const N: usize>(reader: &mut impl Read) -> Result<[u8; N], String> {
    let mut bytes = [0; N];
    reader
        .read_exact(&mut bytes)
        .map_err(|_| "差分文件被截断")?;
    Ok(bytes)
}
pub(super) fn apply(
    base: &Path,
    patch: &Path,
    target: &Path,
    size: u64,
    cancel: &AtomicBool,
) -> Result<(), String> {
    if !(1..=LIMIT).contains(&size) {
        return Err("差分目标大小超限".into());
    }
    let mut source = File::open(base).map_err(|_| "旧程序不可读")?;
    let source_size = source.metadata().map_err(|_| "旧程序不可读")?.len();
    let mut delta = File::open(patch).map_err(|_| "差分文件不可读")?;
    if &number::<8>(&mut delta)? != MAGIC || u64::from_le_bytes(number(&mut delta)?) != size {
        return Err("差分格式或目标大小不匹配".into());
    }
    let mut output = File::options()
        .write(true)
        .create_new(true)
        .open(target)
        .map_err(|_| "差分目标不可写")?;
    let mut written = 0u64;
    let mut operations = 0;
    let mut buffer = [0u8; 65536];
    while written < size {
        if cancel.load(Ordering::Relaxed) {
            return Err("下载已取消".into());
        }
        operations += 1;
        if operations > 1_000_000 {
            return Err("差分操作数超限".into());
        }
        let kind = number::<1>(&mut delta)?[0];
        let offset = if kind == 1 {
            u64::from_le_bytes(number(&mut delta)?)
        } else {
            0
        };
        let length = u32::from_le_bytes(number(&mut delta)?) as u64;
        if length == 0
            || length > size - written
            || kind > 1
            || (kind == 0 && length > buffer.len() as u64)
            || (kind == 1
                && offset
                    .checked_add(length)
                    .is_none_or(|end| end > source_size))
        {
            return Err("差分指令越界".into());
        }
        let input: &mut dyn Read = if kind == 1 {
            source
                .seek(SeekFrom::Start(offset))
                .map_err(|_| "旧程序读取失败")?;
            &mut source
        } else {
            &mut delta
        };
        let mut remaining = length;
        while remaining > 0 {
            if cancel.load(Ordering::Relaxed) {
                return Err("下载已取消".into());
            }
            let count = remaining.min(buffer.len() as u64) as usize;
            input
                .read_exact(&mut buffer[..count])
                .map_err(|_| "差分数据缺失")?;
            output
                .write_all(&buffer[..count])
                .map_err(|_| "重建程序写入失败")?;
            remaining -= count as u64;
        }
        written += length;
    }
    if delta.read(&mut buffer[..1]).map_err(|_| "差分尾部不可读")? != 0 {
        return Err("差分含额外指令".into());
    }
    output.sync_all().map_err(|_| "重建程序同步失败".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn binary_diff_handles_shifted_blocks_changes_and_rejects_wrong_size_and_truncation() {
        let root = super::super::tests::temporary();
        let source: Vec<u8> = (0..20000)
            .map(|i| ((i * 37 + i / 256) % 251) as u8)
            .collect();
        let mut target = b"new prefix changes all following offsets".to_vec();
        target.extend(&source);
        target[12000..12020].fill(42);
        fs::write(root.join("base"), &source).unwrap();
        fs::write(root.join("new"), &target).unwrap();
        create(&root.join("base"), &root.join("new"), &root.join("patch")).unwrap();
        let cancel = AtomicBool::new(false);
        apply(
            &root.join("base"),
            &root.join("patch"),
            &root.join("result"),
            target.len() as u64,
            &cancel,
        )
        .unwrap();
        assert_eq!(fs::read(root.join("result")).unwrap(), target);
        assert!(fs::metadata(root.join("patch")).unwrap().len() < target.len() as u64 / 2);
        assert!(
            apply(
                &root.join("base"),
                &root.join("patch"),
                &root.join("bad-size"),
                LIMIT + 1,
                &cancel
            )
            .is_err()
        );
        let mut bytes = fs::read(root.join("patch")).unwrap();
        bytes.truncate(18);
        fs::write(root.join("truncated"), bytes).unwrap();
        assert!(
            apply(
                &root.join("base"),
                &root.join("truncated"),
                &root.join("bad-patch"),
                target.len() as u64,
                &cancel
            )
            .is_err()
        );
        fs::remove_dir_all(root).unwrap();
    }
}
