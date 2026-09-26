use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::Result;

/// `site.toml` の `timezone` で指定するタイムゾーン。
#[derive(Debug, Clone, PartialEq)]
pub enum TimeZone {
    /// UTC からの時差（分）。
    Offset(i32),
    /// IANA のタイムゾーン名（OS のタイムゾーンデータベースで解決する）。
    Named(String),
}

/// 1970-01-01 からの日数を暦の日付に直す（Howard Hinnant の civil_from_days）。
pub fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let month = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let year = yoe + era * 400 + i64::from(month <= 2);
    (year, month, day)
}

fn format(year: i64, month: u32, day: u32) -> String {
    format!("{year:04}-{month:02}-{day:02}")
}

fn now_secs() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs() as i64)
}

/// UNIX 時刻を、UTC からの時差（分）を足した日付にする。
pub fn date_at(unix_secs: i64, offset_minutes: i32) -> String {
    let (y, m, d) = civil_from_days((unix_secs + i64::from(offset_minutes) * 60).div_euclid(86_400));
    format(y, m, d)
}

pub fn utc_today() -> String {
    date_at(now_secs(), 0)
}

/// 環境のタイムゾーンでの今日の日付。取得できなければ UTC の日付を返す。
pub fn local_today() -> String {
    local::today().unwrap_or_else(utc_today)
}

/// 指定したタイムゾーンでの今日の日付。`None` なら環境のタイムゾーンを使う。
pub fn today_in(timezone: Option<&TimeZone>) -> Result<String> {
    match timezone {
        None => Ok(local_today()),
        Some(TimeZone::Offset(minutes)) => Ok(date_at(now_secs(), *minutes)),
        Some(TimeZone::Named(name)) => local::today_in_zone(name),
    }
}

pub fn parse_timezone(s: &str) -> std::result::Result<TimeZone, String> {
    if s == "UTC" {
        return Ok(TimeZone::Offset(0));
    }
    if let Some(rest) = s.strip_prefix(['+', '-']) {
        let b = rest.as_bytes();
        let shape = b.len() == 5 && b[2] == b':' && [0, 1, 3, 4].iter().all(|&i| b[i].is_ascii_digit());
        if shape {
            let hours: i32 = rest[0..2].parse().unwrap();
            let minutes: i32 = rest[3..5].parse().unwrap();
            let total = hours * 60 + minutes;
            if minutes < 60 && total <= 14 * 60 {
                return Ok(TimeZone::Offset(if s.starts_with('-') { -total } else { total }));
            }
        }
        return Err(format!("{s} は時差として解釈できません（+09:00 のように ±HH:MM で指定してください）"));
    }
    let is_name = !s.is_empty()
        && s.split('/').all(|part| {
            !part.is_empty()
                && part != "."
                && part != ".."
                && part.bytes().all(|b| b.is_ascii_alphanumeric() || b"_+-".contains(&b))
        });
    if is_name {
        Ok(TimeZone::Named(s.to_string()))
    } else {
        Err(format!(
            "{s} はタイムゾーンとして解釈できません（Asia/Tokyo のような名前か、+09:00 のような時差で指定してください）"
        ))
    }
}

pub fn is_valid_date(s: &str) -> bool {
    let b = s.as_bytes();
    let shape = b.len() == 10
        && b.iter().enumerate().all(|(i, c)| match i {
            4 | 7 => *c == b'-',
            _ => c.is_ascii_digit(),
        });
    if !shape {
        return false;
    }
    let year: i64 = s[0..4].parse().unwrap();
    let month: u32 = s[5..7].parse().unwrap();
    let day: u32 = s[8..10].parse().unwrap();
    let leap = (year % 4 == 0 && year % 100 != 0) || year % 400 == 0;
    let days_in_month = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap => 29,
        2 => 28,
        _ => return false,
    };
    (1..=days_in_month).contains(&day)
}

#[cfg(unix)]
mod local {
    use std::ffi::{c_char, c_int, c_long};
    use std::io::Read;
    use std::path::{Path, PathBuf};

    use anyhow::{anyhow, bail, Result};

    #[allow(non_camel_case_types)]
    type time_t = c_long;

    // struct tm の先頭 9 個の int は POSIX で順序が決まっている。
    // glibc、musl、macOS はその後に tm_gmtoff と tm_zone を持つ。末尾の余白は、実装ごとの差を吸収するため。
    #[repr(C)]
    struct Tm {
        tm_sec: c_int,
        tm_min: c_int,
        tm_hour: c_int,
        tm_mday: c_int,
        tm_mon: c_int,
        tm_year: c_int,
        tm_wday: c_int,
        tm_yday: c_int,
        tm_isdst: c_int,
        tm_gmtoff: c_long,
        tm_zone: *const c_char,
        _reserved: [u8; 64],
    }

    extern "C" {
        fn tzset();
        fn time(out: *mut time_t) -> time_t;
        fn localtime_r(time: *const time_t, result: *mut Tm) -> *mut Tm;
    }

    pub fn today() -> Option<String> {
        // SAFETY: tzset と time は引数なし、またはヌルを許す。localtime_r は結果を十分な大きさの Tm に書き込む。
        unsafe {
            tzset();
            let now = time(std::ptr::null_mut());
            let mut tm: Tm = std::mem::zeroed();
            if localtime_r(&now, &mut tm).is_null() {
                return None;
            }
            let date = super::format(i64::from(tm.tm_year) + 1900, (tm.tm_mon + 1) as u32, tm.tm_mday as u32);
            super::is_valid_date(&date).then_some(date)
        }
    }

    const ZONEINFO_DIRS: &[&str] = &["/usr/share/zoneinfo", "/usr/lib/zoneinfo", "/usr/share/lib/zoneinfo"];

    fn zoneinfo_dirs() -> Vec<PathBuf> {
        let mut dirs: Vec<PathBuf> = std::env::var_os("TZDIR").map(PathBuf::from).into_iter().collect();
        dirs.extend(ZONEINFO_DIRS.iter().map(PathBuf::from));
        dirs
    }

    fn is_tzif(path: &Path) -> bool {
        let mut magic = [0u8; 4];
        std::fs::File::open(path).and_then(|mut f| f.read_exact(&mut magic)).is_ok() && &magic == b"TZif"
    }

    /// タイムゾーン名に対応するタイムゾーンデータベースのファイルを探す。
    pub fn find_zoneinfo(name: &str, dirs: &[PathBuf]) -> Option<PathBuf> {
        dirs.iter().map(|dir| dir.join(name)).find(|path| is_tzif(path))
    }

    pub fn today_in_zone(name: &str) -> Result<String> {
        let path = find_zoneinfo(name, &zoneinfo_dirs()).ok_or_else(|| {
            anyhow!(
                "タイムゾーン {name} がこの環境のタイムゾーンデータベースに見つかりません。\
                 名前を確かめるか、+09:00 のような時差で指定してください"
            )
        })?;

        // このプロセスは new の処理中に他のスレッドを持たないので、TZ を一時的に書き換えてよい。
        let previous = std::env::var_os("TZ");
        std::env::set_var("TZ", &path);
        let date = today();
        match previous {
            Some(value) => std::env::set_var("TZ", value),
            None => std::env::remove_var("TZ"),
        }
        // SAFETY: tzset は引数を取らず、元に戻した TZ を読み直すだけ。
        unsafe { tzset() };

        match date {
            Some(date) => Ok(date),
            None => bail!("タイムゾーン {name} での日付を求められませんでした"),
        }
    }
}

#[cfg(not(unix))]
mod local {
    use anyhow::{bail, Result};

    pub fn today() -> Option<String> {
        None
    }

    pub fn today_in_zone(name: &str) -> Result<String> {
        bail!("この環境ではタイムゾーン名（{name}）に対応していません。+09:00 のような時差で指定してください")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_days_to_civil_dates() {
        assert_eq!(civil_from_days(0), (1970, 1, 1));
        assert_eq!(civil_from_days(-1), (1969, 12, 31));
        assert_eq!(civil_from_days(11_016), (2000, 2, 29));
        assert_eq!(civil_from_days(20_722), (2026, 9, 26));
        assert_eq!(civil_from_days(20_819), (2027, 1, 1));
    }

    #[test]
    fn applies_utc_offsets() {
        // 2026-09-25T20:00:00Z
        let t = 20_721 * 86_400 + 20 * 3600;
        assert_eq!(date_at(t, 0), "2026-09-25");
        assert_eq!(date_at(t, 9 * 60), "2026-09-26");
        assert_eq!(date_at(t, 3 * 60 + 59), "2026-09-25");
        assert_eq!(date_at(t, -12 * 60), "2026-09-25");
        assert_eq!(date_at(0, -60), "1969-12-31");
    }

    #[test]
    fn parses_offsets_and_utc() {
        assert_eq!(parse_timezone("UTC"), Ok(TimeZone::Offset(0)));
        assert_eq!(parse_timezone("+09:00"), Ok(TimeZone::Offset(540)));
        assert_eq!(parse_timezone("-05:30"), Ok(TimeZone::Offset(-330)));
        assert_eq!(parse_timezone("+14:00"), Ok(TimeZone::Offset(840)));
        for ng in ["+9:00", "+0900", "+09:60", "+14:30", "+15:00", "09:00x", "+"] {
            assert!(parse_timezone(ng).is_err(), "{ng}");
        }
    }

    #[test]
    fn parses_zone_names() {
        for ok in ["Asia/Tokyo", "America/Argentina/Buenos_Aires", "Etc/GMT+12", "Europe/London"] {
            assert_eq!(parse_timezone(ok), Ok(TimeZone::Named(ok.to_string())), "{ok}");
        }
        for ng in ["", "/etc/passwd", "Asia/../etc", "Asia//Tokyo", "Asia/Tokyo/", "Asia Tokyo", "東京"] {
            assert!(parse_timezone(ng).is_err(), "{ng}");
        }
    }

    #[cfg(unix)]
    #[test]
    fn finds_only_tzif_files_in_zoneinfo_dirs() {
        let first = crate::testutil::tempdir();
        let second = crate::testutil::tempdir();
        std::fs::create_dir_all(first.path().join("Asia")).unwrap();
        std::fs::write(first.path().join("Asia/Fake"), b"not tz data").unwrap();
        std::fs::create_dir_all(second.path().join("Asia")).unwrap();
        std::fs::write(second.path().join("Asia/Fake"), b"TZif2...").unwrap();

        let dirs = vec![first.path().to_path_buf(), second.path().to_path_buf()];
        assert_eq!(local::find_zoneinfo("Asia/Fake", &dirs), Some(second.path().join("Asia/Fake")));
        assert_eq!(local::find_zoneinfo("Asia/Missing", &dirs), None);
    }

    #[test]
    fn validates_calendar_dates() {
        for ok in ["2026-09-26", "2024-02-29", "2000-02-29", "1999-12-31"] {
            assert!(is_valid_date(ok), "{ok}");
        }
        for ng in ["2026-9-26", "2026/09/26", "2025-02-29", "1900-02-29", "2026-13-01", "2026-04-31", "2026-00-10", "2026-01-00"] {
            assert!(!is_valid_date(ng), "{ng}");
        }
    }

    #[test]
    fn today_is_a_valid_date_close_to_utc() {
        let local = local_today();
        let utc = utc_today();
        assert!(is_valid_date(&local), "{local}");
        assert!(is_valid_date(&utc), "{utc}");
        let to_days = |s: &str| {
            let (y, m, d): (i64, i64, i64) = (s[0..4].parse().unwrap(), s[5..7].parse().unwrap(), s[8..10].parse().unwrap());
            y * 372 + m * 31 + d
        };
        assert!((to_days(&local) - to_days(&utc)).abs() <= 31, "{local} と {utc} が離れすぎています");
    }
}
