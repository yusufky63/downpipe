//! File names of site downloads from the user's template
//! (Settings, Media: `{title}`, `{channel}`, `{date}`, `{index}`, `{id}`).
//!
//! The template is Downpipe's own, not yt-dlp's output template: Downpipe
//! fixes the name before yt-dlp starts (so a resume finds its parts and a
//! second download of the same video gets a free name), and a template the
//! user typed never reaches yt-dlp's `%(...)s` syntax. The desktop app
//! previews it with a copy of these rules (`apps/desktop/src/lib/naming.ts`),
//! checked against the same fixture (`tests/fixtures/naming.json`).

/// The name when the user set nothing: the video's title.
pub const DEFAULT_TEMPLATE: &str = "{title}";

/// Longest template kept, in characters.
pub const MAX_TEMPLATE: usize = 200;

/// What a template can name a video after. Empty fields drop out together
/// with the separators around them.
#[derive(Debug, Clone, Copy, Default)]
pub struct NameFields<'a> {
    pub title: Option<&'a str>,
    /// The channel or uploader.
    pub channel: Option<&'a str>,
    /// yt-dlp's `upload_date`, `YYYYMMDD`.
    pub upload_date: Option<&'a str>,
    /// Position in the playlist it was added from, 1 first.
    pub index: Option<u32>,
    /// Videos in that playlist, for zero padding (`07` of 42).
    pub count: Option<u32>,
    pub id: Option<&'a str>,
}

/// Characters that only separate parts: ` - `, `_`, `.`, `,`, `|`, `·`.
fn is_separator(c: char) -> bool {
    c.is_whitespace()
        || matches!(
            c,
            '-' | '_' | '.' | ',' | '|' | '\u{b7}' | '\u{2013}' | '\u{2014}'
        )
}

enum Part {
    Text(String),
    Field(String),
}

/// The template with the fields filled in, before it is made a safe file
/// name. Unknown `{words}` stay as they are. Text made only of separators
/// is kept only between two parts that are not empty, so `{index} - {title}`
/// of a single video is just its title.
pub fn render(template: &str, fields: &NameFields<'_>) -> String {
    let mut parts: Vec<Part> = Vec::new();
    let mut text = String::new();
    let mut rest = template;
    while let Some(open) = rest.find('{') {
        text.push_str(&rest[..open]);
        let after = &rest[open + 1..];
        let value = after
            .find('}')
            .and_then(|close| field(&after[..close], fields).map(|v| (close, v)));
        match value {
            Some((close, value)) => {
                if !text.is_empty() {
                    parts.push(Part::Text(std::mem::take(&mut text)));
                }
                parts.push(Part::Field(value));
                rest = &after[close + 1..];
            }
            None => {
                text.push('{');
                rest = after;
            }
        }
    }
    text.push_str(rest);
    if !text.is_empty() {
        parts.push(Part::Text(text));
    }

    // The first separator after something filled waits for the next filled
    // part; empty fields and further separators in between drop out.
    let mut out = String::new();
    let mut pending: Option<&str> = None;
    for part in &parts {
        let value = match part {
            Part::Text(t) if t.chars().all(is_separator) => {
                if !out.is_empty() && pending.is_none() {
                    pending = Some(t);
                }
                continue;
            }
            Part::Text(t) => t.as_str(),
            Part::Field(v) if v.is_empty() => continue,
            Part::Field(v) => v.as_str(),
        };
        if let Some(sep) = pending.take() {
            out.push_str(sep);
        }
        out.push_str(value);
    }
    out.trim().to_string()
}

fn field(name: &str, f: &NameFields<'_>) -> Option<String> {
    let clean = |v: Option<&str>| v.map(|v| v.trim().to_string()).unwrap_or_default();
    Some(match name {
        "title" => clean(f.title),
        "channel" => clean(f.channel),
        "id" => clean(f.id),
        "date" => f.upload_date.map(date).unwrap_or_default(),
        "index" => match f.index {
            Some(i) => {
                let width = f.count.unwrap_or(i).max(i).to_string().len();
                format!("{i:0width$}")
            }
            None => String::new(),
        },
        _ => return None,
    })
}

/// `20050423` as `2005-04-23`; anything else as nothing.
fn date(raw: &str) -> String {
    let raw = raw.trim();
    if raw.len() == 8 && raw.bytes().all(|b| b.is_ascii_digit()) {
        format!("{}-{}-{}", &raw[..4], &raw[4..6], &raw[6..])
    } else {
        String::new()
    }
}

/// A template as stored: trimmed, capped, the default when empty.
pub fn normalize(template: &str) -> String {
    let t: String = template
        .trim()
        .chars()
        .filter(|c| !c.is_control())
        .take(MAX_TEMPLATE)
        .collect();
    let t = t.trim();
    if t.is_empty() {
        DEFAULT_TEMPLATE.to_string()
    } else {
        t.to_string()
    }
}

/// The stem a site download is saved under: the rendered template as a
/// safe file name, or the title when the template came out empty.
pub fn stem(template: &str, fields: &NameFields<'_>) -> String {
    let rendered = render(template, fields);
    let name = if rendered.is_empty() {
        fields.title.unwrap_or_default().to_string()
    } else {
        rendered
    };
    crate::files::stem_from_title(&name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;

    #[derive(Deserialize)]
    struct Fields {
        title: Option<String>,
        channel: Option<String>,
        upload_date: Option<String>,
        index: Option<u32>,
        count: Option<u32>,
        id: Option<String>,
    }

    #[derive(Deserialize)]
    struct Case {
        template: String,
        fields: Fields,
        expected: String,
    }

    /// The same cases the desktop app's `naming.test.ts` reads.
    #[test]
    fn shared_fixture() {
        let path =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/naming.json");
        let cases: Vec<Case> = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
        assert!(cases.len() > 5);
        for case in cases {
            let f = &case.fields;
            let fields = NameFields {
                title: f.title.as_deref(),
                channel: f.channel.as_deref(),
                upload_date: f.upload_date.as_deref(),
                index: f.index,
                count: f.count,
                id: f.id.as_deref(),
            };
            assert_eq!(
                render(&case.template, &fields),
                case.expected,
                "{}",
                case.template
            );
        }
    }

    #[test]
    fn stems_are_safe_names() {
        let fields = NameFields {
            title: Some("a/b: c?"),
            ..NameFields::default()
        };
        assert_eq!(
            stem("{title}", &fields),
            crate::files::stem_from_title("a/b: c?")
        );
        // Nothing to fill in: the title.
        assert_eq!(stem("{channel}", &fields), stem("{title}", &fields));
        assert_eq!(stem("{channel}", &NameFields::default()), "video");
    }

    #[test]
    fn normalizes() {
        assert_eq!(normalize("  "), DEFAULT_TEMPLATE);
        assert_eq!(normalize(" {index} - {title}\n"), "{index} - {title}");
        assert_eq!(normalize(&"x".repeat(300)).chars().count(), MAX_TEMPLATE);
    }
}
