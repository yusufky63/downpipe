//! Command lines the user types, run as a program with an argument list.
//!
//! The text is split into words once, when it is read, by Downpipe's own
//! rules (spaces separate, double or single quotes group, nothing else is
//! special). Placeholders are filled in afterwards, inside each word, so a
//! file name or URL with spaces, quotes, `&`, `;` or `$(...)` stays one
//! argument and never becomes part of a command. No shell is involved.

use std::path::Path;
use std::process::Stdio;
use std::time::Duration;

use downpipe_providers::process::Tree;
use tokio::io::{AsyncRead, AsyncReadExt};

/// The words a placeholder can stand for.
pub const PLACEHOLDERS: [&str; 4] = ["{path}", "{name}", "{folder}", "{url}"];

/// Output kept per run, from the end, for the report and the log.
const OUTPUT_TAIL: usize = 4 * 1024;

/// A parsed command line: the program and its argument templates.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandLine {
    pub program: String,
    pub args: Vec<String>,
}

/// Why a command line cannot run.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum CommandLineError {
    #[error("the command is empty")]
    Empty,
    #[error("a quote is not closed")]
    UnclosedQuote,
    #[error("the program must be typed out, not a placeholder")]
    PlaceholderProgram,
    #[error("the program must be a name on PATH or a full path, not `{0}`")]
    RelativeProgram(String),
    #[error("the command contains a control character")]
    ControlCharacter,
}

/// What the placeholders stand for in one run.
#[derive(Debug, Clone, Default)]
pub struct Values {
    pub path: String,
    pub name: String,
    pub folder: String,
    pub url: String,
}

/// How a run ended.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Exit {
    /// The program ran and exited; `None` when a signal ended it.
    Code(Option<i32>),
    /// It ran longer than allowed and was stopped, with everything it started.
    TimedOut,
    /// It could not be started.
    NotStarted(std::io::ErrorKind, String),
}

/// One finished run.
#[derive(Debug, Clone)]
pub struct Run {
    pub exit: Exit,
    /// The end of stdout and stderr, in that order, decoded lossily.
    pub output: String,
}

impl CommandLine {
    /// Split `line` into a program and arguments.
    ///
    /// The program must be what the user typed: a bare name (looked up on
    /// `PATH`, never in the download folder) or an absolute path. A relative
    /// path such as `tools\x.exe` is refused because it would be resolved
    /// against a folder the user does not see.
    pub fn parse(line: &str) -> Result<Self, CommandLineError> {
        if line.chars().any(|c| c.is_control() && c != '\t') {
            return Err(CommandLineError::ControlCharacter);
        }
        let mut words = split(line)?.into_iter();
        let program = words.next().ok_or(CommandLineError::Empty)?;
        if PLACEHOLDERS.iter().any(|p| program.contains(p)) {
            return Err(CommandLineError::PlaceholderProgram);
        }
        let has_separator = program.contains(['/', '\\']);
        if has_separator && !Path::new(&program).is_absolute() {
            return Err(CommandLineError::RelativeProgram(program));
        }
        Ok(Self {
            program,
            args: words.collect(),
        })
    }

    /// Check a command line from the settings or a dialog; empty is fine.
    pub fn validate(line: &str) -> Result<(), CommandLineError> {
        if line.trim().is_empty() {
            return Ok(());
        }
        Self::parse(line).map(|_| ())
    }

    /// The arguments with the placeholders filled in, one argument each.
    pub fn expand(&self, values: &Values) -> Vec<String> {
        self.args.iter().map(|a| fill(a, values)).collect()
    }

    /// Whether any argument mentions `placeholder`.
    pub fn mentions(&self, placeholder: &str) -> bool {
        self.args.iter().any(|a| a.contains(placeholder))
    }

    /// The program's file name, for reports ("7z.exe").
    pub fn program_name(&self) -> String {
        Path::new(&self.program).file_name().map_or_else(
            || self.program.clone(),
            |n| n.to_string_lossy().into_owned(),
        )
    }
}

/// Words separated by spaces or tabs; `"..."` and `'...'` group and are
/// removed. Backslashes are ordinary characters, as in Windows paths.
fn split(line: &str) -> Result<Vec<String>, CommandLineError> {
    let mut words = Vec::new();
    let mut word = String::new();
    // A word exists once something was typed, even `""` (an empty argument).
    let mut in_word = false;
    let mut quote: Option<char> = None;
    for c in line.chars() {
        match quote {
            Some(q) if c == q => quote = None,
            Some(_) => word.push(c),
            None => match c {
                '"' | '\'' => {
                    quote = Some(c);
                    in_word = true;
                }
                ' ' | '\t' => {
                    if in_word {
                        words.push(std::mem::take(&mut word));
                        in_word = false;
                    }
                }
                c => {
                    word.push(c);
                    in_word = true;
                }
            },
        }
    }
    if quote.is_some() {
        return Err(CommandLineError::UnclosedQuote);
    }
    if in_word {
        words.push(word);
    }
    Ok(words)
}

/// Replace the placeholders in one word. Values are inserted as they are;
/// nothing in them is read as a placeholder again.
fn fill(word: &str, values: &Values) -> String {
    let mut out = String::with_capacity(word.len());
    let mut rest = word;
    'outer: while !rest.is_empty() {
        for (placeholder, value) in [
            ("{path}", &values.path),
            ("{name}", &values.name),
            ("{folder}", &values.folder),
            ("{url}", &values.url),
        ] {
            if let Some(after) = rest.strip_prefix(placeholder) {
                out.push_str(value);
                rest = after;
                continue 'outer;
            }
        }
        let mut chars = rest.chars();
        if let Some(c) = chars.next() {
            out.push(c);
        }
        rest = chars.as_str();
    }
    out
}

/// Run `program` with `args` and wait at most `timeout`. stdin is closed,
/// there is no console window, and the working folder stays Downpipe's own:
/// never the download folder, where a downloaded DLL or script could be
/// picked up by the program.
pub async fn run(program: &str, args: &[String], timeout: Duration) -> Run {
    let mut command = tokio::process::Command::new(program);
    command
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    #[cfg(windows)]
    {
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    #[cfg(unix)]
    command.process_group(0);

    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(err) => {
            return Run {
                exit: Exit::NotStarted(err.kind(), err.to_string()),
                output: String::new(),
            };
        }
    };
    // Whatever the program starts is stopped with it on a timeout, and on
    // Windows also when Downpipe goes away.
    let tree = Tree::adopt(&child);
    let stdout = child.stdout.take();
    let stderr = child.stderr.take();
    let read = async {
        let (out, err) = tokio::join!(tail(stdout), tail(stderr));
        (out, err)
    };
    let work = async {
        let (status, (out, err)) = tokio::join!(child.wait(), read);
        (status, out, err)
    };
    let result = tokio::time::timeout(timeout, work).await;
    match result {
        Ok((status, out, err)) => Run {
            exit: match status {
                Ok(status) => Exit::Code(status.code()),
                Err(err) => Exit::NotStarted(err.kind(), err.to_string()),
            },
            output: join_output(&out, &err),
        },
        Err(_) => {
            tree.kill(&mut child);
            let _ = child.wait().await;
            Run {
                exit: Exit::TimedOut,
                output: String::new(),
            }
        }
    }
}

/// Read a stream to its end, keeping only its last bytes, so a chatty
/// program cannot fill the memory.
async fn tail<R: AsyncRead + Unpin>(stream: Option<R>) -> Vec<u8> {
    let Some(mut stream) = stream else {
        return Vec::new();
    };
    let mut kept = Vec::new();
    let mut buf = [0u8; 8192];
    loop {
        match stream.read(&mut buf).await {
            Ok(0) | Err(_) => break,
            Ok(n) => {
                kept.extend_from_slice(&buf[..n]);
                if kept.len() > OUTPUT_TAIL * 2 {
                    kept.drain(..kept.len() - OUTPUT_TAIL);
                }
            }
        }
    }
    if kept.len() > OUTPUT_TAIL {
        kept.drain(..kept.len() - OUTPUT_TAIL);
    }
    kept
}

fn join_output(out: &[u8], err: &[u8]) -> String {
    let out = String::from_utf8_lossy(out);
    let err = String::from_utf8_lossy(err);
    let text = match (out.trim().is_empty(), err.trim().is_empty()) {
        (false, false) => format!("{}\n{}", out.trim_end(), err.trim_end()),
        (false, true) => out.trim_end().to_string(),
        (true, false) => err.trim_end().to_string(),
        (true, true) => String::new(),
    };
    // Control characters other than line breaks and tabs (colours, cursor
    // moves) are dropped: the text is shown and logged as it is.
    text.chars()
        .filter(|&c| !c.is_control() || c == '\n' || c == '\t')
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn values() -> Values {
        Values {
            path: r"C:\Downloads\a b.zip".into(),
            name: "a b.zip".into(),
            folder: r"C:\Downloads".into(),
            url: "https://example.org/a%20b.zip?x=1&y=2".into(),
        }
    }

    #[test]
    fn splits_words_and_quotes() {
        let line = CommandLine::parse(
            r#""C:\Program Files\7-Zip\7z.exe" x {path} "-o{folder}\out" '' -y"#,
        )
        .unwrap();
        assert_eq!(line.program, r"C:\Program Files\7-Zip\7z.exe");
        assert_eq!(line.args, ["x", "{path}", r"-o{folder}\out", "", "-y"]);
        assert_eq!(line.program_name(), "7z.exe");
    }

    #[test]
    fn placeholders_stay_one_argument() {
        let line = CommandLine::parse("notify-send Done {name} --url={url}").unwrap();
        assert_eq!(
            line.expand(&values()),
            [
                "Done",
                "a b.zip",
                "--url=https://example.org/a%20b.zip?x=1&y=2"
            ]
        );
        // A name made to look like a command or another placeholder is
        // passed as text.
        let hostile = Values {
            name: r#"x"; rm -rf ~; echo "{path}"#.into(),
            ..values()
        };
        assert_eq!(
            line.expand(&hostile)[1],
            r#"x"; rm -rf ~; echo "{path}"#,
            "values are not expanded again"
        );
    }

    #[test]
    fn refuses_what_cannot_run_safely() {
        assert_eq!(CommandLine::parse("   "), Err(CommandLineError::Empty));
        assert_eq!(
            CommandLine::parse(r#""C:\x.exe {path}"#),
            Err(CommandLineError::UnclosedQuote)
        );
        assert_eq!(
            CommandLine::parse("{path} --flag"),
            Err(CommandLineError::PlaceholderProgram)
        );
        assert_eq!(
            CommandLine::parse(r"tools\run.exe {path}"),
            Err(CommandLineError::RelativeProgram(r"tools\run.exe".into()))
        );
        assert_eq!(
            CommandLine::parse("echo a\nrm b"),
            Err(CommandLineError::ControlCharacter)
        );
        assert!(CommandLine::validate("").is_ok());
        assert!(CommandLine::parse("notepad {path}").is_ok());
    }

    #[test]
    fn output_keeps_the_end_without_control_characters() {
        let long = "x".repeat(OUTPUT_TAIL * 3);
        let text = join_output(long.as_bytes(), b"\x1b[31merror\x1b[0m\r\n");
        assert!(text.ends_with("[31merror[0m"), "{text:?}");
        assert!(!text.contains('\x1b') && !text.contains('\r'));
    }
}
