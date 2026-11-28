//! Append-only JSONL journal of translator lookups in `~/Hutarka/lookups/`.

use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::translator::{Kind, Lang};

#[derive(Debug, Clone, Serialize)]
pub struct LookupRecord {
    /// RFC 3339, local time.
    pub ts: String,
    /// Always "lookup".
    pub kind: &'static str,
    pub mode: Kind,
    /// The target language.
    pub to: Lang,
    pub text: String,
    pub result: String,
    pub model: String,
    pub ms: u64,
}

/// `$HOME/Hutarka`.
pub fn journal_root() -> PathBuf {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_default()
        .join("Hutarka")
}

/// Appends one line to `root/lookups/YYYY-MM-DD.jsonl`; returns its path.
pub fn append_lookup(
    root: &Path,
    rec: &LookupRecord,
    date: chrono::NaiveDate,
) -> std::io::Result<PathBuf> {
    let dir = root.join("lookups");
    std::fs::create_dir_all(&dir)?;
    let path = dir.join(format!("{}.jsonl", date.format("%Y-%m-%d")));
    let mut line = serde_json::to_string(rec)?;
    line.push('\n');
    let mut file = OpenOptions::new().create(true).append(true).open(&path)?;
    // One write per line, so lines from two writers never interleave.
    file.write_all(line.as_bytes())?;
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::translator::{Kind, Lang};

    fn record(text: &str) -> LookupRecord {
        LookupRecord {
            ts: "2026-11-27T14:30:00+01:00".into(),
            kind: "lookup",
            mode: Kind::Word,
            to: Lang::Ru,
            text: text.into(),
            result: "рычаг".into(),
            model: "claude-haiku-5-5".into(),
            ms: 812,
        }
    }

    fn date() -> chrono::NaiveDate {
        chrono::NaiveDate::from_ymd_opt(2026, 11, 27).unwrap()
    }

    #[test]
    fn creates_dir_and_appends_line() {
        let root = tempfile::tempdir().unwrap();
        let path = append_lookup(root.path(), &record("leverage"), date()).unwrap();
        append_lookup(root.path(), &record("gain"), date()).unwrap();
        assert_eq!(path, root.path().join("lookups/2026-11-27.jsonl"));
        let content = std::fs::read_to_string(&path).unwrap();
        let lines: Vec<serde_json::Value> = content
            .lines()
            .map(|l| serde_json::from_str(l).unwrap())
            .collect();
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0]["kind"], "lookup");
        assert_eq!(lines[0]["mode"], "word");
        assert_eq!(lines[0]["to"], "ru");
        assert_eq!(lines[1]["text"], "gain");
        assert_eq!(lines[1]["ms"], 812);
    }

    #[test]
    fn multiline_text_stays_one_line() {
        let root = tempfile::tempdir().unwrap();
        let path = append_lookup(root.path(), &record("one\ntwo\n"), date()).unwrap();
        let content = std::fs::read_to_string(path).unwrap();
        assert_eq!(content.lines().count(), 1);
        assert!(content.ends_with('\n'));
    }
}
