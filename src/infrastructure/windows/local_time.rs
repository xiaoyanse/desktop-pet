use crate::interfaces::LocalTime;
use windows_sys::Win32::{
    Foundation::{FILETIME, SYSTEMTIME},
    System::Time::{
        FileTimeToSystemTime, SystemTimeToFileTime, SystemTimeToTzSpecificLocalTime,
        TzSpecificLocalTimeToSystemTime,
    },
};
const UNIX_OFFSET: u64 = 116444736000000000;
pub struct WindowsLocalTime;
fn local(seconds: u64) -> Option<SYSTEMTIME> {
    let ticks = seconds.checked_mul(10_000_000)?.checked_add(UNIX_OFFSET)?;
    let file = FILETIME {
        dwLowDateTime: ticks as u32,
        dwHighDateTime: (ticks >> 32) as u32,
    };
    let mut utc = SYSTEMTIME::default();
    let mut local = SYSTEMTIME::default();
    if unsafe { FileTimeToSystemTime(&file, &mut utc) } == 0
        || unsafe { SystemTimeToTzSpecificLocalTime(std::ptr::null(), &utc, &mut local) } == 0
    {
        return None;
    }
    Some(local)
}
impl LocalTime for WindowsLocalTime {
    fn format(&self, seconds: u64) -> String {
        local(seconds).map_or_else(
            || "时间超出范围".into(),
            |t| {
                format!(
                    "{:04}-{:02}-{:02} {:02}:{:02}",
                    t.wYear, t.wMonth, t.wDay, t.wHour, t.wMinute
                )
            },
        )
    }
    fn parse(&self, text: &str) -> Result<u64, String> {
        let text = text.trim();
        let b = text.as_bytes();
        if b.len() != 16
            || b[4] != b'-'
            || b[7] != b'-'
            || b[10] != b' '
            || b[13] != b':'
            || !b
                .iter()
                .enumerate()
                .all(|(i, c)| [4, 7, 10, 13].contains(&i) || c.is_ascii_digit())
        {
            return Err("请输入本地时间 YYYY-MM-DD HH:mm".into());
        }
        let number = |start: usize, end: usize| text[start..end].parse::<u16>().unwrap();
        let input = SYSTEMTIME {
            wYear: number(0, 4),
            wMonth: number(5, 7),
            wDay: number(8, 10),
            wHour: number(11, 13),
            wMinute: number(14, 16),
            ..Default::default()
        };
        if !(1970..=9999).contains(&input.wYear)
            || !(1..=12).contains(&input.wMonth)
            || !(1..=31).contains(&input.wDay)
            || input.wHour > 23
            || input.wMinute > 59
        {
            return Err("日期或时间无效".into());
        }
        let mut utc = SYSTEMTIME::default();
        let mut ft = FILETIME::default();
        if unsafe { TzSpecificLocalTimeToSystemTime(std::ptr::null(), &input, &mut utc) } == 0
            || unsafe { SystemTimeToFileTime(&utc, &mut ft) } == 0
        {
            return Err("日期或时间无效".into());
        }
        let ticks = (ft.dwHighDateTime as u64) << 32 | ft.dwLowDateTime as u64;
        let seconds = ticks.checked_sub(UNIX_OFFSET).ok_or("时间不能早于1970年")? / 10_000_000;
        if self.format(seconds) != text {
            return Err("日期不存在或落在夏令时跳变期间".into());
        }
        Ok(seconds)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn local_date_roundtrip_and_invalid_dates() {
        let clock = WindowsLocalTime;
        for text in ["2026-10-12 09:30", "2028-02-29 15:00", "2030-01-01 00:01"] {
            assert_eq!(clock.format(clock.parse(text).unwrap()), text);
        }
        for text in [
            "2027-02-29 09:30",
            "2026-10-32 10:00",
            "2026-13-01 01:00",
            "2026-10-12 24:00",
            "2026-10-12 09:60",
            "你好你好你好你好",
            "2026/10/12 09:30",
        ] {
            assert!(clock.parse(text).is_err(), "{text}");
        }
    }
}
