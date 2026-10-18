//! Links in an HTML page, found with a small tolerant tokenizer.
//!
//! A full HTML parser would be a large dependency for what the grabber needs:
//! the `href` and `src` attributes of a handful of elements. The tokenizer
//! skips comments and the contents of `<script>`, `<style>` and `<textarea>`
//! (whose text may look like tags), reads quoted and unquoted attribute
//! values, decodes the common character references and honours `<base>`.

use url::Url;

/// What kind of element a link came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinkKind {
    /// `<a>`, `<area>`, a `<meta http-equiv="refresh">` target: a page or a
    /// file the user would click.
    Navigation,
    /// `<iframe>` / `<frame>`: another page shown inside this one.
    Frame,
    /// `<img>`, `<video>`, `<audio>`, `<source>`, `<embed>`, `<object>`,
    /// `<track>`, `<link>`: something the page loads.
    Embedded,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Link {
    pub url: Url,
    pub kind: LinkKind,
}

/// Every http(s) link of `html`, resolved against `page` (or the page's
/// `<base href>`), without fragments, in document order and without
/// duplicates.
pub fn links(html: &str, page: &Url) -> Vec<Link> {
    let mut base = page.clone();
    let mut base_set = false;
    let mut out: Vec<Link> = Vec::new();
    let bytes = html.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] != b'<' {
            i += 1;
            continue;
        }
        if html[i..].starts_with("<!--") {
            i = html[i + 4..]
                .find("-->")
                .map_or(bytes.len(), |p| i + 4 + p + 3);
            continue;
        }
        let Some(tag) = read_tag(html, i) else {
            i += 1;
            continue;
        };
        i = tag.end;
        if tag.closing {
            continue;
        }
        let name = tag.name.as_str();
        if matches!(name, "script" | "style" | "textarea" | "title") {
            let close = format!("</{name}");
            i = find_ci(html, i, &close).unwrap_or(bytes.len());
            continue;
        }
        let (attr, kind) = match name {
            "base" => {
                if !base_set
                    && let Some(href) = tag.attr("href")
                    && let Ok(u) = page.join(href.trim())
                {
                    base = u;
                    base_set = true;
                }
                continue;
            }
            "a" | "area" => ("href", LinkKind::Navigation),
            "link" => ("href", LinkKind::Embedded),
            "iframe" | "frame" => ("src", LinkKind::Frame),
            "img" | "video" | "audio" | "source" | "embed" | "track" => ("src", LinkKind::Embedded),
            "object" => ("data", LinkKind::Embedded),
            "meta" => {
                let refresh = tag
                    .attr("http-equiv")
                    .is_some_and(|v| v.eq_ignore_ascii_case("refresh"));
                if refresh && let Some(target) = tag.attr("content").and_then(refresh_target) {
                    push(&mut out, &base, &target, LinkKind::Navigation);
                }
                continue;
            }
            _ => continue,
        };
        if let Some(value) = tag.attr(attr) {
            push(&mut out, &base, value, kind);
        }
        // A video's poster is an image the page shows, like an <img>.
        if name == "video"
            && let Some(poster) = tag.attr("poster")
        {
            push(&mut out, &base, poster, LinkKind::Embedded);
        }
    }
    out
}

fn push(out: &mut Vec<Link>, base: &Url, value: &str, kind: LinkKind) {
    let value = value.trim();
    if value.is_empty() || value.starts_with('#') {
        return;
    }
    let Ok(mut url) = base.join(value) else {
        return;
    };
    if !matches!(url.scheme(), "http" | "https") {
        return;
    }
    url.set_fragment(None);
    match out.iter_mut().find(|l| l.url == url) {
        // A link the user can click says more than the same file embedded.
        Some(existing) if kind == LinkKind::Navigation => existing.kind = kind,
        Some(_) => {}
        None => out.push(Link { url, kind }),
    }
}

/// `5; url=/next` -> `/next`.
fn refresh_target(content: &str) -> Option<String> {
    let (_, rest) = content.split_once(';')?;
    let rest = rest.trim();
    rest.get(..4).filter(|p| p.eq_ignore_ascii_case("url="))?;
    let target = rest[4..].trim().trim_matches(|c| c == '\'' || c == '"');
    (!target.is_empty()).then(|| target.to_string())
}

fn find_ci(haystack: &str, from: usize, needle: &str) -> Option<usize> {
    let lower = needle.to_ascii_lowercase();
    let hay = haystack.as_bytes();
    let n = lower.as_bytes();
    (from..hay.len().saturating_sub(n.len() - 1))
        .find(|&p| hay[p..p + n.len()].eq_ignore_ascii_case(n))
}

#[derive(Debug)]
struct Tag {
    name: String,
    closing: bool,
    attrs: Vec<(String, String)>,
    /// Byte offset just past the `>`.
    end: usize,
}

impl Tag {
    fn attr(&self, name: &str) -> Option<&str> {
        self.attrs
            .iter()
            .find(|(k, _)| k == name)
            .map(|(_, v)| v.as_str())
    }
}

/// The tag starting at `start` (which holds `<`), if it is one.
fn read_tag(html: &str, start: usize) -> Option<Tag> {
    let b = html.as_bytes();
    let mut i = start + 1;
    let closing = b.get(i) == Some(&b'/');
    if closing {
        i += 1;
    }
    let name_start = i;
    while i < b.len() && (b[i].is_ascii_alphanumeric() || b[i] == b'-' || b[i] == b':') {
        i += 1;
    }
    if i == name_start || !b[name_start].is_ascii_alphabetic() {
        return None;
    }
    let name = html[name_start..i].to_ascii_lowercase();
    let mut attrs = Vec::new();
    loop {
        while i < b.len() && (b[i].is_ascii_whitespace() || b[i] == b'/') {
            i += 1;
        }
        if i >= b.len() {
            return Some(Tag {
                name,
                closing,
                attrs,
                end: b.len(),
            });
        }
        if b[i] == b'>' {
            return Some(Tag {
                name,
                closing,
                attrs,
                end: i + 1,
            });
        }
        let key_start = i;
        while i < b.len() && !b[i].is_ascii_whitespace() && !matches!(b[i], b'=' | b'>' | b'/') {
            i += 1;
        }
        let key = html[key_start..i].to_ascii_lowercase();
        while i < b.len() && b[i].is_ascii_whitespace() {
            i += 1;
        }
        let mut value = String::new();
        if b.get(i) == Some(&b'=') {
            i += 1;
            while i < b.len() && b[i].is_ascii_whitespace() {
                i += 1;
            }
            match b.get(i) {
                Some(&q @ (b'"' | b'\'')) => {
                    let v_start = i + 1;
                    let v_end = html[v_start..]
                        .find(q as char)
                        .map_or(b.len(), |p| v_start + p);
                    value = decode_entities(&html[v_start..v_end]);
                    i = (v_end + 1).min(b.len());
                }
                _ => {
                    let v_start = i;
                    while i < b.len() && !b[i].is_ascii_whitespace() && b[i] != b'>' {
                        i += 1;
                    }
                    value = decode_entities(&html[v_start..i]);
                }
            }
        }
        if key.is_empty() {
            // A stray character such as `"`: skip it rather than loop.
            i += 1;
        } else if !attrs.iter().any(|(k, _)| *k == key) {
            attrs.push((key, value));
        }
    }
}

/// The character references that show up in URLs: `&amp;`, `&quot;`,
/// `&apos;`, `&lt;`, `&gt;`, `&nbsp;` and numeric ones.
pub fn decode_entities(s: &str) -> String {
    if !s.contains('&') {
        return s.to_string();
    }
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(p) = rest.find('&') {
        out.push_str(&rest[..p]);
        rest = &rest[p..];
        let end = rest[1..]
            .find(|c: char| !(c.is_ascii_alphanumeric() || c == '#'))
            .map(|e| e + 1);
        let (name, after) = match end {
            Some(e) if rest[e..].starts_with(';') => (&rest[1..e], &rest[e + 1..]),
            Some(e) => (&rest[1..e], &rest[e..]),
            None => (&rest[1..], ""),
        };
        let decoded = match name {
            "amp" => Some('&'),
            "quot" => Some('"'),
            "apos" => Some('\''),
            "lt" => Some('<'),
            "gt" => Some('>'),
            "nbsp" => Some('\u{a0}'),
            n if n.starts_with("#x") || n.starts_with("#X") => u32::from_str_radix(&n[2..], 16)
                .ok()
                .and_then(char::from_u32),
            n if n.starts_with('#') => n[1..].parse().ok().and_then(char::from_u32),
            _ => None,
        };
        match decoded {
            Some(c) => {
                out.push(c);
                rest = after;
            }
            None => {
                out.push('&');
                rest = &rest[1..];
            }
        }
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn urls(html: &str) -> Vec<String> {
        links(
            html,
            &Url::parse("https://site.test/dir/page.html").unwrap(),
        )
        .into_iter()
        .map(|l| l.url.to_string())
        .collect()
    }

    #[test]
    fn finds_links_of_the_elements_that_matter() {
        let html = r##"<html><head><link rel=stylesheet href="/s.css"><title>a <a href="no.zip"></title></head>
            <body><A HREF='one.zip'>One</A> <a href=two.pdf#page=2>Two</a>
            <a href="https://other.test/x.mp3">x</a> <a href="mailto:a@b.test">m</a> <a href="javascript:void(0)">j</a>
            <img src="pic.jpg" alt="a > b"> <video src="v.mp4" poster="p.png"><source src="v.webm"></video>
            <iframe src="frame.html"></iframe><a href="#top">top</a><a href="  sub/  ">sub</a></body></html>"##;
        assert_eq!(
            urls(html),
            [
                "https://site.test/s.css",
                "https://site.test/dir/one.zip",
                "https://site.test/dir/two.pdf",
                "https://other.test/x.mp3",
                "https://site.test/dir/pic.jpg",
                "https://site.test/dir/v.mp4",
                "https://site.test/dir/p.png",
                "https://site.test/dir/v.webm",
                "https://site.test/dir/frame.html",
                "https://site.test/dir/sub/",
            ]
        );
    }

    #[test]
    fn skips_comments_and_scripts() {
        let html = r#"<!-- <a href="c.zip"> --><script>var s = '<a href="s.zip">';</script>
            <style>a { background: url(<a href="st.zip">) }</style><a href="real.zip">r</a>"#;
        assert_eq!(urls(html), ["https://site.test/dir/real.zip"]);
    }

    #[test]
    fn honours_base_entities_and_refresh() {
        let html = r#"<base href="https://cdn.test/files/"><a href="a.zip?x=1&amp;y=2">a</a>
            <meta http-equiv="Refresh" content="0; URL='/next.html'"><a href="&#x62;.zip">b</a>"#;
        assert_eq!(
            urls(html),
            [
                "https://cdn.test/files/a.zip?x=1&y=2",
                "https://cdn.test/next.html",
                "https://cdn.test/files/b.zip"
            ]
        );
    }

    #[test]
    fn a_clickable_link_wins_over_an_embedded_one() {
        let page = Url::parse("https://site.test/").unwrap();
        let found = links(r#"<img src="a.png"><a href="a.png">a</a>"#, &page);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].kind, LinkKind::Navigation);
    }

    #[test]
    fn survives_broken_markup() {
        for html in [
            "<",
            "<a",
            "<a href=",
            "<a href='x",
            "<a \"\"\" href=x.zip>",
            "<<a>>",
            "<!--",
            "<script>",
            "&#xffffffff; &bogus; &",
        ] {
            let _ = urls(html);
        }
        assert_eq!(urls("<a \"\" href=x.zip>"), ["https://site.test/dir/x.zip"]);
        assert_eq!(
            decode_entities("&#xffffffff; &bogus; & &lt;"),
            "&#xffffffff; &bogus; & <"
        );
    }
}
