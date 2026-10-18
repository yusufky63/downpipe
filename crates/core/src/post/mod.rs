//! What happens after a download completes: a virus scan, extracting an
//! archive, and a command the user typed, in that order.
//!
//! Nothing here runs for a download that failed, was paused or canceled, and
//! nothing runs that the user did not set: the scanner is Defender or the
//! user's command, and commands come from the settings, a queue or the
//! download itself, never from a server or the browser.
//!
//! * [`scan`] runs the scanner and reads its verdict from the exit code.
//! * [`extract`] unpacks zip, tar (`.gz`, `.xz`) and 7z archives safely.
//! * [`command`] parses command lines and runs programs.
//!
//! A threat found stops the rest: the archive is not extracted and the
//! command does not run. Nothing is ever deleted because of a scan.

pub mod command;
pub mod extract;
pub mod scan;

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::time::Duration;

use serde::{Deserialize, Serialize};

use self::command::{CommandLine, CommandLineError, Exit, Values};
use self::extract::{ArchiveKind, Limits};
use self::scan::Scanner;
use crate::settings::Settings;
use crate::task::TaskRecord;

/// Post-download choices made for one download. Empty values follow its
/// queue, its category and the settings.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct PostActions {
    /// Command line to run when it completes. Empty: the queue's, else the
    /// one in the settings.
    pub command: String,
    /// Extract it when it is an archive. `None`: its category decides.
    pub extract: Option<bool>,
}

impl PostActions {
    pub(crate) fn normalized(mut self) -> Self {
        self.command = self.command.trim().to_string();
        self
    }
}

/// The settings' part: defaults for every download.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct PostSettings {
    /// Command line run after every completed download whose queue or own
    /// choice sets none. Empty: none.
    pub command: String,
    /// Seconds a command or a scan may take before it is stopped.
    pub timeout_secs: u32,
    /// Scan completed downloads. Off by default.
    pub scan: bool,
    /// Scanner command line. Empty: Microsoft Defender on Windows; nothing
    /// elsewhere.
    pub scanner: String,
    /// Delete an archive once everything in it was extracted.
    pub delete_archive: bool,
}

impl Default for PostSettings {
    fn default() -> Self {
        Self {
            command: String::new(),
            timeout_secs: 600,
            scan: false,
            scanner: String::new(),
            delete_archive: false,
        }
    }
}

impl PostSettings {
    pub(crate) fn normalized(mut self) -> Self {
        self.command = self.command.trim().to_string();
        self.scanner = self.scanner.trim().to_string();
        self.timeout_secs = self.timeout_secs.clamp(5, 86_400);
        self
    }
}

/// The step running now, shown while the download is otherwise complete.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PostStep {
    Scanning,
    Extracting,
    RunningCommand,
}

/// What the steps found, kept with the download.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct PostReport {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scan: Option<ScanReport>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extract: Option<ExtractReport>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command: Option<CommandReport>,
    /// The scanner found a threat, so the archive was not extracted and the
    /// command did not run.
    pub stopped_by_threat: bool,
}

impl PostReport {
    pub fn threat_found(&self) -> bool {
        self.scan
            .as_ref()
            .is_some_and(|s| s.result == ScanResult::Threat)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScanResult {
    Clean,
    Threat,
    /// No scanner: Defender is not installed, the program was not found, or
    /// none is set on this system.
    Missing,
    /// The scanner ran but gave no verdict (an error, a timeout).
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScanReport {
    pub result: ScanResult,
    /// "Microsoft Defender", or the program's file name.
    pub scanner: String,
    /// The threat's name, or why the scan failed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExtractResult {
    Done,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExtractReport {
    pub result: ExtractResult,
    /// The new folder with the files.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub folder: Option<PathBuf>,
    #[serde(default)]
    pub files: u64,
    #[serde(default)]
    pub bytes: u64,
    /// Entries left out: unsafe names, links, special files, duplicates.
    #[serde(default)]
    pub skipped: u64,
    #[serde(default)]
    pub archive_deleted: bool,
    /// `encrypted`, `too_large`, `too_many_entries`, `corrupt`, `io`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CommandResult {
    /// Exit code 0.
    Ok,
    /// Another exit code, or ended by a signal.
    Failed,
    TimedOut,
    /// The program could not be started.
    NotStarted,
    /// The command line cannot run (for example an unclosed quote).
    Invalid,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandReport {
    pub result: CommandResult,
    /// The program's file name.
    pub program: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<i32>,
    /// The end of what it printed.
    #[serde(default)]
    pub output: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

/// Validate every command line in the settings, for `update_settings`.
pub(crate) fn validate_settings(settings: &Settings) -> Result<(), String> {
    CommandLine::validate(&settings.post.command)
        .map_err(|e| format!("command after download: {e}"))?;
    CommandLine::validate(&settings.post.scanner).map_err(|e| format!("virus scanner: {e}"))?;
    for q in &settings.queues {
        CommandLine::validate(&q.command)
            .map_err(|e| format!("command of the queue `{}`: {e}", q.name))?;
    }
    Ok(())
}

pub(crate) fn validate_actions(actions: &PostActions) -> Result<(), CommandLineError> {
    CommandLine::validate(&actions.command)
}

/// The steps one completed download takes, decided when it completes.
#[derive(Debug)]
pub(crate) struct Plan {
    path: PathBuf,
    values: Values,
    scan: Option<Result<Scanner, CommandLineError>>,
    extract: Option<ArchiveKind>,
    delete_archive: bool,
    command: Option<Result<CommandLine, CommandLineError>>,
    timeout: Duration,
}

impl Plan {
    /// `None` when nothing is to be done.
    pub(crate) fn new(settings: &Settings, record: &TaskRecord) -> Option<Self> {
        // A relative download folder in the settings gives a relative path,
        // which MpCmdRun refuses and other programs resolve against a
        // folder of their own: always hand them an absolute one.
        let path = record.path.clone()?;
        let path = std::path::absolute(&path).unwrap_or(path);
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let command_line = [
            record.post_actions.command.as_str(),
            settings
                .queue(&record.queue)
                .map_or("", |q| q.command.as_str()),
            settings.post.command.as_str(),
        ]
        .into_iter()
        .map(str::trim)
        .find(|c| !c.is_empty());
        let extract = ArchiveKind::of(&name).filter(|_| {
            record.post_actions.extract.unwrap_or_else(|| {
                settings
                    .category_of(&name)
                    .is_some_and(|category| category.extract)
            })
        });
        let plan = Self {
            // A folder typed with `/` on Windows gives paths that some
            // programs cannot open; they get the system's separators.
            values: Values {
                path: scan::native_separators(&path.to_string_lossy()),
                folder: path
                    .parent()
                    .map(|p| scan::native_separators(&p.to_string_lossy()))
                    .unwrap_or_default(),
                name,
                // The stored URL: credentials are already removed, so they
                // never show up in a process list.
                url: record.url.clone(),
            },
            path,
            scan: settings
                .post
                .scan
                .then(|| Scanner::from_setting(&settings.post.scanner)),
            extract,
            delete_archive: settings.post.delete_archive,
            command: command_line.map(CommandLine::parse),
            timeout: Duration::from_secs(u64::from(settings.post.timeout_secs.max(5))),
        };
        (plan.scan.is_some() || plan.extract.is_some() || plan.command.is_some()).then_some(plan)
    }

    pub(crate) fn first_step(&self) -> PostStep {
        if self.scan.is_some() {
            PostStep::Scanning
        } else if self.extract.is_some() {
            PostStep::Extracting
        } else {
            PostStep::RunningCommand
        }
    }
}

/// What the runner reports as it goes.
#[derive(Debug)]
pub(crate) enum Update {
    Step(PostStep),
    Scan(ScanReport),
    Extract(ExtractReport),
    Command(CommandReport),
    StoppedByThreat,
    ArchiveDeleted,
    Done,
}

/// Run `plan`, reporting each step to `apply`, which answers `false` when
/// the download is gone and the rest should not run.
pub(crate) async fn run(
    id: &str,
    plan: Plan,
    cancel: Arc<AtomicBool>,
    apply: impl Fn(Update) -> bool,
) {
    let mut extracted_whole = false;
    if let Some(scanner) = &plan.scan {
        if !apply(Update::Step(PostStep::Scanning)) {
            return;
        }
        let report = match scanner {
            Ok(scanner) => scanner.scan(&plan.path, &plan.values, plan.timeout).await,
            Err(e) => ScanReport {
                result: ScanResult::Failed,
                scanner: String::new(),
                detail: Some(e.to_string()),
            },
        };
        let threat = report.result == ScanResult::Threat;
        if threat {
            tracing::warn!(
                task = id,
                threat = report.detail.as_deref().unwrap_or("?"),
                "the scanner found a threat in {}; nothing else runs",
                plan.path.display()
            );
        }
        if !apply(Update::Scan(report)) {
            return;
        }
        if threat {
            if plan.extract.is_some() || plan.command.is_some() {
                apply(Update::StoppedByThreat);
            }
            apply(Update::Done);
            return;
        }
    }
    if let Some(kind) = plan.extract {
        if !apply(Update::Step(PostStep::Extracting)) {
            return;
        }
        let archive = plan.path.clone();
        let flag = cancel.clone();
        let result = tokio::task::spawn_blocking(move || {
            extract::extract(&archive, kind, Limits::default(), &flag)
        })
        .await;
        let report = match result {
            Ok(Ok(done)) => {
                tracing::info!(
                    task = id,
                    files = done.files,
                    bytes = done.bytes,
                    skipped = done.skipped,
                    "extracted {} into {}",
                    plan.path.display(),
                    done.folder.display()
                );
                extracted_whole = done.skipped == 0;
                ExtractReport {
                    result: ExtractResult::Done,
                    folder: Some(done.folder),
                    files: done.files,
                    bytes: done.bytes,
                    skipped: done.skipped,
                    archive_deleted: false,
                    error: None,
                    detail: None,
                }
            }
            // Stopped on purpose: whoever stopped it cleans up.
            Ok(Err(extract::ExtractError::Canceled)) => return,
            Ok(Err(err)) => {
                tracing::warn!(
                    task = id,
                    "could not extract {}: {err}",
                    plan.path.display()
                );
                failed_extract(err.code(), &err.to_string())
            }
            // The blocking thread panicked: still a report, so the download
            // does not show as extracting forever.
            Err(join) => failed_extract("io", &join.to_string()),
        };
        if !apply(Update::Extract(report)) {
            return;
        }
    }
    if let Some(line) = &plan.command {
        if !apply(Update::Step(PostStep::RunningCommand)) {
            return;
        }
        let report = run_command(id, line, &plan).await;
        if !apply(Update::Command(report)) {
            return;
        }
    }
    // Last, so the command still finds the archive at `{path}`. An archive
    // with entries left out is kept: it has more than the folder.
    if extracted_whole && plan.delete_archive {
        match std::fs::remove_file(&plan.path) {
            Ok(()) => {
                apply(Update::ArchiveDeleted);
            }
            Err(err) => tracing::warn!(
                task = id,
                "could not delete {} after extracting it: {err}",
                plan.path.display()
            ),
        }
    }
    apply(Update::Done);
}

fn failed_extract(code: &str, detail: &str) -> ExtractReport {
    ExtractReport {
        result: ExtractResult::Failed,
        folder: None,
        files: 0,
        bytes: 0,
        skipped: 0,
        archive_deleted: false,
        error: Some(code.to_string()),
        detail: Some(detail.chars().take(300).collect()),
    }
}

async fn run_command(
    id: &str,
    line: &Result<CommandLine, CommandLineError>,
    plan: &Plan,
) -> CommandReport {
    let line = match line {
        Ok(line) => line,
        Err(e) => {
            return CommandReport {
                result: CommandResult::Invalid,
                program: String::new(),
                exit_code: None,
                output: String::new(),
                detail: Some(e.to_string()),
            };
        }
    };
    let args = line.expand(&plan.values);
    let run = command::run(&line.program, &args, plan.timeout).await;
    tracing::info!(
        task = id,
        program = line.program,
        exit = ?run.exit,
        "post-download command finished; output:\n{}",
        run.output
    );
    let (result, exit_code, detail) = match run.exit {
        Exit::Code(Some(0)) => (CommandResult::Ok, Some(0), None),
        Exit::Code(code) => (CommandResult::Failed, code, None),
        Exit::TimedOut => (CommandResult::TimedOut, None, None),
        Exit::NotStarted(_, message) => (CommandResult::NotStarted, None, Some(message)),
    };
    CommandReport {
        result,
        program: line.program_name(),
        exit_code,
        output: run.output,
        detail,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::Queue;
    use crate::task::TaskState;

    fn record(name: &str) -> TaskRecord {
        TaskRecord {
            id: "t".into(),
            url: "https://example.org/x".into(),
            dir: PathBuf::from("/d"),
            file_name: Some(name.into()),
            path: Some(PathBuf::from("/d").join(name)),
            state: TaskState::Completed,
            total: None,
            downloaded: 0,
            error: None,
            error_kind: None,
            checksum: None,
            max_connections: None,
            category: None,
            auto_dir: false,
            queue: "night".into(),
            position: 0,
            media: None,
            site: None,
            post_actions: PostActions::default(),
            post: None,
            created_at: 0,
            updated_at: 0,
            completed_at: None,
        }
    }

    fn settings() -> Settings {
        let mut s = Settings::default();
        s.queues.push(Queue {
            id: "night".into(),
            name: "Night".into(),
            ..Queue::default()
        });
        s
    }

    fn command_of(plan: &Plan) -> String {
        plan.command
            .as_ref()
            .unwrap()
            .as_ref()
            .unwrap()
            .program
            .clone()
    }

    #[test]
    fn nothing_to_do_by_default() {
        assert!(Plan::new(&Settings::default(), &record("a.zip")).is_none());
    }

    #[test]
    fn the_download_then_the_queue_then_the_settings_choose_the_command() {
        let mut s = settings();
        s.post.command = "global".into();
        let mut r = record("a.bin");
        assert_eq!(command_of(&Plan::new(&s, &r).unwrap()), "global");
        s.queues[1].command = "queued".into();
        assert_eq!(command_of(&Plan::new(&s, &r).unwrap()), "queued");
        r.post_actions.command = "mine {path}".into();
        let plan = Plan::new(&s, &r).unwrap();
        assert_eq!(command_of(&plan), "mine");
        assert_eq!(plan.first_step(), PostStep::RunningCommand);
        assert_eq!(plan.values.name, "a.bin");
        assert_eq!(plan.values.url, "https://example.org/x");
    }

    #[test]
    fn categories_and_downloads_choose_extraction() {
        let mut s = settings();
        let mut r = record("a.tar.gz");
        assert!(Plan::new(&s, &r).is_none(), "off by default");
        let compressed = s
            .categories
            .iter_mut()
            .find(|c| c.name == "Compressed")
            .unwrap();
        compressed.extract = true;
        let plan = Plan::new(&s, &r).unwrap();
        assert_eq!(plan.extract, Some(ArchiveKind::TarGz));
        assert_eq!(plan.first_step(), PostStep::Extracting);
        // Still the category's type when sorting into folders is off.
        s.use_categories = false;
        assert!(Plan::new(&s, &r).is_some());
        r.post_actions.extract = Some(false);
        assert!(
            Plan::new(&s, &r).is_none(),
            "the download's own choice wins"
        );
        let mut rar = record("a.rar");
        rar.post_actions.extract = Some(true);
        assert!(Plan::new(&s, &rar).is_none(), "not a format Downpipe opens");
    }

    #[test]
    fn scanning_comes_first() {
        let mut s = settings();
        s.post.scan = true;
        s.post.command = "x".into();
        let plan = Plan::new(&s, &record("a.exe")).unwrap();
        assert_eq!(plan.first_step(), PostStep::Scanning);
    }

    #[test]
    fn report_round_trips_without_empty_parts() {
        let report = PostReport {
            scan: Some(ScanReport {
                result: ScanResult::Threat,
                scanner: scan::DEFENDER.into(),
                detail: Some("EICAR".into()),
            }),
            stopped_by_threat: true,
            ..PostReport::default()
        };
        let json = serde_json::to_string(&report).unwrap();
        assert_eq!(
            json,
            r#"{"scan":{"result":"threat","scanner":"Microsoft Defender","detail":"EICAR"},"stopped_by_threat":true}"#
        );
        assert_eq!(serde_json::from_str::<PostReport>(&json).unwrap(), report);
        assert!(report.threat_found());
    }

    #[test]
    fn settings_validation_names_the_bad_field() {
        let mut s = settings();
        assert!(validate_settings(&s).is_ok());
        s.queues[1].command = "\"unclosed".into();
        assert!(validate_settings(&s).unwrap_err().contains("Night"));
    }
}
