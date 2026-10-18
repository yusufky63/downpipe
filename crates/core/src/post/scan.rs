//! Virus scan of a finished download with a scanner that is already on the
//! computer: Microsoft Defender on Windows, or a command the user sets.
//!
//! The scanner only reports. Defender runs with `-DisableRemediation`, so it
//! does not quarantine or delete the file either; what to do with a file a
//! scanner flags is the user's decision.

use std::path::{Path, PathBuf};
use std::time::Duration;

use super::command::{self, CommandLine, Exit, Values};
use super::{ScanReport, ScanResult};

/// The scanner a download is checked with.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Scanner {
    /// `MpCmdRun.exe` at this path (which may not exist).
    Defender(PathBuf),
    /// The user's command; its exit code says what it found.
    Custom(CommandLine),
    /// Nothing is set on a system without Defender.
    None,
}

/// Defender's own name, shown with the result.
pub const DEFENDER: &str = "Microsoft Defender";

impl Scanner {
    /// The scanner for the `scanner` setting: that command when set, else
    /// Defender on Windows.
    pub fn from_setting(scanner: &str) -> Result<Self, command::CommandLineError> {
        if !scanner.trim().is_empty() {
            return CommandLine::parse(scanner).map(Self::Custom);
        }
        Ok(if cfg!(windows) {
            Self::Defender(defender_path())
        } else {
            Self::None
        })
    }

    fn name(&self) -> String {
        match self {
            Self::Defender(_) => DEFENDER.into(),
            Self::Custom(line) => line.program_name(),
            Self::None => String::new(),
        }
    }

    /// Scan `path`; `values` fill the custom command's placeholders. A
    /// custom command that does not mention `{path}` gets it as its last
    /// argument.
    pub async fn scan(&self, path: &Path, values: &Values, timeout: Duration) -> ScanReport {
        let scanner = self.name();
        let (program, args, verdict) = match self {
            Self::None => return report(ScanResult::Missing, scanner, None),
            Self::Defender(exe) => {
                if !exe.is_file() {
                    return report(ScanResult::Missing, scanner, None);
                }
                let args = [
                    "-Scan",
                    "-ScanType",
                    "3",
                    "-File",
                    // MpCmdRun cannot find a file named with `/`
                    // (0x80508023), which a folder typed by hand can give.
                    &native_separators(&path.to_string_lossy()),
                    "-DisableRemediation",
                ]
                .map(String::from)
                .to_vec();
                (exe.to_string_lossy().into_owned(), args, Verdict::Defender)
            }
            Self::Custom(line) => {
                let mut args = line.expand(values);
                if !line.mentions("{path}") {
                    args.push(values.path.clone());
                }
                (line.program.clone(), args, Verdict::ExitOne)
            }
        };
        let run = command::run(&program, &args, timeout).await;
        tracing::info!(
            scanner,
            exit = ?run.exit,
            "scanned {}: {}",
            path.display(),
            run.output.lines().last().unwrap_or_default()
        );
        interpret(&run.exit, &run.output, verdict, scanner)
    }
}

/// How a scanner's exit code reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Verdict {
    /// MpCmdRun: 0 nothing found, 2 a threat found, but also 2 for its own
    /// errors (`CmdTool: Failed with hr = 0x...`), so a threat needs the
    /// threat list in its output too.
    Defender,
    /// ClamAV's convention, which most command-line scanners follow: 0
    /// clean, 1 infected, anything else an error.
    ExitOne,
}

/// `C:/a/b` as `C:\a\b` on Windows; unchanged elsewhere.
pub(crate) fn native_separators(path: &str) -> String {
    if cfg!(windows) {
        path.replace('/', "\\")
    } else {
        path.to_string()
    }
}

/// Defender listed a threat: its `LIST OF DETECTED THREATS` block or a
/// `Threat : name` line, or `found N threats` with N above zero.
fn defender_found_threat(output: &str) -> bool {
    output.lines().map(str::trim).any(|line| {
        line.contains("LIST OF DETECTED THREATS")
            || line
                .strip_prefix("Threat")
                .is_some_and(|rest| rest.trim_start().starts_with(':'))
            || line.split_once(" found ").is_some_and(|(_, rest)| {
                rest.ends_with("threats.")
                    && rest
                        .split_whitespace()
                        .next()
                        .and_then(|n| n.parse::<u64>().ok())
                        .is_some_and(|n| n > 0)
            })
    })
}

/// `%ProgramFiles%\Windows Defender\MpCmdRun.exe`.
pub fn defender_path() -> PathBuf {
    let base = std::env::var_os("ProgramFiles")
        .filter(|v| !v.is_empty())
        .map_or_else(|| PathBuf::from(r"C:\Program Files"), PathBuf::from);
    base.join("Windows Defender").join("MpCmdRun.exe")
}

fn interpret(exit: &Exit, output: &str, verdict: Verdict, scanner: String) -> ScanReport {
    let threat = match (verdict, exit) {
        (Verdict::Defender, Exit::Code(Some(2))) => defender_found_threat(output),
        (Verdict::ExitOne, Exit::Code(Some(1))) => true,
        _ => false,
    };
    match exit {
        Exit::Code(Some(0)) => report(ScanResult::Clean, scanner, None),
        _ if threat => report(ScanResult::Threat, scanner, threat_name(output)),
        Exit::Code(code) => report(
            ScanResult::Failed,
            scanner,
            Some(last_line(output).unwrap_or_else(|| match code {
                Some(code) => format!("exit code {code}"),
                None => "ended by a signal".into(),
            })),
        ),
        Exit::TimedOut => report(ScanResult::Failed, scanner, Some("timed out".into())),
        Exit::NotStarted(std::io::ErrorKind::NotFound, _) => {
            report(ScanResult::Missing, scanner, None)
        }
        Exit::NotStarted(_, message) => report(ScanResult::Failed, scanner, Some(message.clone())),
    }
}

fn report(result: ScanResult, scanner: String, detail: Option<String>) -> ScanReport {
    ScanReport {
        result,
        scanner,
        detail: detail.map(|d| cap(&d)),
    }
}

/// The name of what was found: Defender's `Threat : Virus:DOS/EICAR...`
/// line, ClamAV's `file: Eicar-Signature FOUND`, else the last line.
fn threat_name(output: &str) -> Option<String> {
    for line in output.lines().map(str::trim) {
        if let Some(rest) = line.strip_prefix("Threat")
            && let Some((_, name)) = rest.split_once(':')
            && !rest.trim_start().starts_with("s ")
        {
            let name = name.trim();
            if !name.is_empty() {
                return Some(name.to_string());
            }
        }
        if let Some(found) = line.strip_suffix(" FOUND")
            && let Some((_, name)) = found.rsplit_once(": ")
        {
            return Some(name.trim().to_string());
        }
    }
    last_line(output)
}

fn last_line(output: &str) -> Option<String> {
    output
        .lines()
        .map(str::trim)
        .rfind(|l| !l.is_empty())
        .map(str::to_string)
}

fn cap(text: &str) -> String {
    text.chars().take(200).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exit_codes_decide() {
        let s = || "scanner".to_string();
        assert_eq!(
            interpret(&Exit::Code(Some(0)), "", Verdict::Defender, s()).result,
            ScanResult::Clean
        );
        let defender = "Scan starting...\nScan finished.\nScanning C:\\x.com found 1 threats.\n\
            <===========================LIST OF DETECTED THREATS==========================>\n\
            ----------------------------- Threat information ------------------------------\n\
            Threat                  : Virus:DOS/EICAR_Test_File\n\
            Resources               : 1 total\n";
        let found = interpret(&Exit::Code(Some(2)), defender, Verdict::Defender, s());
        assert_eq!(found.result, ScanResult::Threat);
        assert_eq!(found.detail.as_deref(), Some("Virus:DOS/EICAR_Test_File"));

        let clam = "/tmp/eicar.com: Win.Test.EICAR_HDB-1 FOUND\n\n----------- SCAN SUMMARY";
        let found = interpret(&Exit::Code(Some(1)), clam, Verdict::ExitOne, s());
        assert_eq!(found.result, ScanResult::Threat);
        assert_eq!(found.detail.as_deref(), Some("Win.Test.EICAR_HDB-1"));

        let failed = interpret(
            &Exit::Code(Some(2)),
            "ERROR: cannot open",
            Verdict::ExitOne,
            s(),
        );
        assert_eq!(failed.result, ScanResult::Failed);
        assert_eq!(failed.detail.as_deref(), Some("ERROR: cannot open"));
        // MpCmdRun's own errors exit with 2 as well (seen with a path
        // written with `/`, and with a missing file): not a threat.
        let error = "Scan starting...\nCmdTool: Failed with hr = 0x80508023. Check C:\\Users\\x\\AppData\\Local\\Temp\\MpCmdRun.log for more information";
        let failed = interpret(&Exit::Code(Some(2)), error, Verdict::Defender, s());
        assert_eq!(failed.result, ScanResult::Failed);
        assert!(failed.detail.unwrap().contains("0x80508023"));
        assert!(!defender_found_threat(
            "Scanning C:\\x.zip found no threats."
        ));
        assert!(defender_found_threat("Scanning C:\\x.com found 1 threats."));
        assert_eq!(
            interpret(
                &Exit::NotStarted(std::io::ErrorKind::NotFound, "nope".into()),
                "",
                Verdict::ExitOne,
                s()
            )
            .result,
            ScanResult::Missing
        );
        assert_eq!(
            interpret(&Exit::TimedOut, "", Verdict::ExitOne, s()).result,
            ScanResult::Failed
        );
    }

    #[test]
    fn defender_by_default_on_windows_only() {
        let scanner = Scanner::from_setting("").unwrap();
        if cfg!(windows) {
            let Scanner::Defender(path) = scanner else {
                panic!("{scanner:?}")
            };
            assert!(path.ends_with(r"Windows Defender\MpCmdRun.exe"));
        } else {
            assert_eq!(scanner, Scanner::None);
        }
        assert!(matches!(
            Scanner::from_setting("clamscan --no-summary").unwrap(),
            Scanner::Custom(_)
        ));
    }

    /// Runs the real Defender on a harmless file; slow, so only on request
    /// (`cargo test -p downpipe-core -- --ignored defender`).
    #[cfg(windows)]
    #[tokio::test]
    #[ignore = "starts Microsoft Defender"]
    async fn defender_finds_a_plain_file_clean() {
        let dir = tempfile::tempdir().unwrap();
        // Written with `/`, as a folder typed into the settings can be.
        let file =
            PathBuf::from(dir.path().to_string_lossy().replace('\\', "/")).join("plain file.txt");
        std::fs::write(&file, b"nothing to see here").unwrap();
        let report = Scanner::Defender(defender_path())
            .scan(&file, &Values::default(), Duration::from_secs(300))
            .await;
        assert_eq!(report.result, ScanResult::Clean, "{report:?}");
    }

    #[tokio::test]
    async fn a_missing_scanner_says_so() {
        let values = Values::default();
        let none = Scanner::None
            .scan(Path::new("x"), &values, Duration::from_secs(5))
            .await;
        assert_eq!(none.result, ScanResult::Missing);
        let gone = Scanner::Defender(PathBuf::from("/nowhere/MpCmdRun.exe"))
            .scan(Path::new("x"), &values, Duration::from_secs(5))
            .await;
        assert_eq!(gone.result, ScanResult::Missing);
        assert_eq!(gone.scanner, DEFENDER);
        let custom = Scanner::from_setting("downpipe-no-such-scanner-here")
            .unwrap()
            .scan(Path::new("x"), &values, Duration::from_secs(5))
            .await;
        assert_eq!(custom.result, ScanResult::Missing);
    }
}
