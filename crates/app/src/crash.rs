//! Crash reports: on a panic, a readable report (version, system, message, call stack,
//! last log lines) goes to `crash-reports/`. At the next launch the title screen offers
//! to report it: a pre-filled GitHub issue opens in the browser. Nothing is ever sent
//! automatically.

use std::collections::VecDeque;
use std::fmt::Write as _;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

const KEPT_LOG_LINES: usize = 60;
const NEW_ISSUE: &str = "https://github.com/SinfulDice/strata-mundi/issues/new";
/// Browsers and GitHub refuse very long addresses: the report is shortened to fit.
const MAX_URL: usize = 7500;
const SEEN_FILE: &str = ".last-seen";

/// The last log lines, kept in memory for the crash report.
#[derive(Clone, Default)]
pub struct RecentLogs(Arc<Mutex<VecDeque<String>>>);

impl RecentLogs {
    fn lines(&self) -> Vec<String> {
        // Never wait in a panic hook: a panic while logging leaves the lock taken.
        self.0
            .try_lock()
            .map(|lines| lines.iter().cloned().collect())
            .unwrap_or_default()
    }
}

impl io::Write for RecentLogs {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if let Ok(mut lines) = self.0.try_lock() {
            for line in String::from_utf8_lossy(bytes).lines() {
                if lines.len() == KEPT_LOG_LINES {
                    lines.pop_front();
                }
                lines.push_back(line.to_owned());
            }
        }
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl<'a> tracing_subscriber::fmt::MakeWriter<'a> for RecentLogs {
    type Writer = RecentLogs;

    fn make_writer(&'a self) -> Self::Writer {
        self.clone()
    }
}

/// Writes a report for any panic, then lets the usual message print.
pub fn install(dir: PathBuf, logs: RecentLogs) {
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let report = report(info, &logs.lines());
        let name = chrono::Local::now()
            .format("crash-%Y-%m-%d_%H-%M-%S.txt")
            .to_string();
        let written =
            std::fs::create_dir_all(&dir).and_then(|()| std::fs::write(dir.join(&name), report));
        if let Err(error) = written {
            eprintln!("could not write the crash report: {error}");
        }
        default_hook(info);
    }));
}

fn report(info: &std::panic::PanicHookInfo<'_>, logs: &[String]) -> String {
    let mut text = String::new();
    let thread = std::thread::current();
    let _ = writeln!(text, "Strata Mundi {} crashed", env!("CARGO_PKG_VERSION"));
    let _ = writeln!(
        text,
        "System: {} {}",
        std::env::consts::OS,
        std::env::consts::ARCH
    );
    let _ = writeln!(text, "Thread: {}", thread.name().unwrap_or("unnamed"));
    let _ = writeln!(text, "Panic: {info}\n");
    let _ = writeln!(
        text,
        "Call stack:\n{}",
        std::backtrace::Backtrace::force_capture()
    );
    let _ = writeln!(text, "Last log lines:");
    for line in logs {
        let _ = writeln!(text, "{line}");
    }
    text
}

/// The newest report not reported or dismissed yet.
pub fn unreported(dir: &Path) -> Option<PathBuf> {
    let newest = std::fs::read_dir(dir)
        .ok()?
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|e| e == "txt"))
        .max()?;
    let seen = std::fs::read_to_string(dir.join(SEEN_FILE)).unwrap_or_default();
    let name = newest.file_name()?.to_string_lossy().into_owned();
    (seen.trim() != name).then_some(newest)
}

/// Remembers that the player saw `report` (reported or dismissed).
pub fn mark_seen(report: &Path) {
    if let (Some(dir), Some(name)) = (report.parent(), report.file_name()) {
        let _ = std::fs::write(dir.join(SEEN_FILE), name.to_string_lossy().as_bytes());
    }
}

/// A new GitHub issue holding the report (shortened to fit in an address).
pub fn issue_url(report: &str) -> String {
    let report = without_home(report);
    let title = report.lines().next().unwrap_or("Crash");
    let mut kept = report.chars().count();
    loop {
        let mut body: String = report.chars().take(kept).collect();
        if kept < report.chars().count() {
            body.push_str("\n[…] (full report in the crash-reports folder)");
        }
        let url = format!(
            "{NEW_ISSUE}?labels=crash&title={}&body={}",
            percent_encode(title),
            percent_encode(&format!("```\n{body}\n```"))
        );
        // The panic message and the top of the call stack come first and are kept.
        if url.len() <= MAX_URL || kept == 0 {
            return url;
        }
        kept = kept * 9 / 10;
    }
}

/// The player's home folder (and so their user name) becomes `~`.
fn without_home(report: &str) -> String {
    match dirs::home_dir().and_then(|home| home.to_str().map(str::to_owned)) {
        Some(home) if !home.is_empty() => report.replace(&home, "~"),
        _ => report.to_owned(),
    }
}

fn percent_encode(text: &str) -> String {
    let mut encoded = String::with_capacity(text.len() * 3);
    for byte in text.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                encoded.push(byte as char);
            }
            _ => {
                let _ = write!(encoded, "%{byte:02X}");
            }
        }
    }
    encoded
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn urls_are_encoded_and_reports_marked_seen() {
        assert_eq!(percent_encode("a b/é"), "a%20b%2F%C3%A9");
        assert!(issue_url("Boom\nline").contains("title=Boom&body=%60%60%60"));
        let long = format!("Boom\n{}", "frame / at src:1\n".repeat(2000));
        assert!(issue_url(&long).len() <= MAX_URL);
        let dir = std::env::temp_dir().join(format!("strata-crash-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("crash-1.txt"), "x").unwrap();
        let report = unreported(&dir).unwrap();
        mark_seen(&report);
        assert_eq!(unreported(&dir), None);
        std::fs::remove_dir_all(dir).unwrap();
    }
}
