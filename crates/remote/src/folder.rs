//! The folder a remote download goes to: only ever a folder inside the
//! download folder, named by the user, never a path of the machine.

use std::path::{Path, PathBuf};

const MAX_LEN: usize = 200;
const MAX_DEPTH: usize = 8;

/// `Movies/2026` inside `base`. Empty means the automatic folder (`None`).
/// Absolute paths, drive letters, `.` and `..`, and names the file system
/// would change are refused rather than repaired, so the user sees why.
pub(crate) fn resolve(base: &Path, folder: &str) -> Result<Option<PathBuf>, ()> {
    let folder = folder.trim();
    if folder.is_empty() {
        return Ok(None);
    }
    if folder.len() > MAX_LEN || folder.starts_with(['/', '\\']) || folder.contains(':') {
        return Err(());
    }
    let mut path = base.to_path_buf();
    let mut depth = 0;
    for part in folder.split(['/', '\\']).map(str::trim) {
        if part.is_empty() {
            continue;
        }
        if part == "." || part == ".." || part.chars().any(char::is_control) {
            return Err(());
        }
        if downpipe_core::http::filename::sanitize(part) != part {
            return Err(());
        }
        depth += 1;
        if depth > MAX_DEPTH {
            return Err(());
        }
        path.push(part);
    }
    Ok((depth > 0).then_some(path))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stays_inside_the_download_folder() {
        let base = Path::new("D:/Downloads");
        assert_eq!(resolve(base, "  "), Ok(None));
        assert_eq!(
            resolve(base, "Movies/2026"),
            Ok(Some(base.join("Movies").join("2026")))
        );
        assert_eq!(
            resolve(base, r"Movies\\Old"),
            Ok(Some(base.join("Movies").join("Old")))
        );
        assert_eq!(resolve(base, "///"), Err(()), "starts at the root");
        for bad in [
            "../Windows",
            "Movies/../../x",
            "./x",
            "/etc",
            r"\\server\share",
            "C:/Windows",
            "C:Windows",
            "a?b",
            "CON",
            "trailing.",
            "x\u{7}y",
            "a/b/c/d/e/f/g/h/i",
        ] {
            assert_eq!(resolve(base, bad), Err(()), "{bad:?}");
        }
        assert_eq!(resolve(base, &"a".repeat(MAX_LEN + 1)), Err(()));
    }
}
