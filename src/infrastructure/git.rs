//! Git and GitHub integration for the file manager: repository detection,
//! GitHub URL mapping and cloning, mirroring what VS Code's Git extension
//! offers from the Explorer.

use std::path::Path;
use std::process::Command;

/// Everything needed to build a github.com URL for a file or directory.
pub struct Repository {
    /// `owner/name` parsed from the origin remote.
    pub slug: String,
    /// The checked-out branch, when one exists (detached heads have none).
    pub branch: Option<String>,
    /// The full commit SHA, for permalinks that never move.
    pub commit: Option<String>,
    /// Path of the repository root.
    pub root: std::path::PathBuf,
}

/// Run git inside `directory`, capturing stdout; None when git is missing or
/// the command fails.
fn git(directory: &Path, args: &[&str]) -> Option<String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(directory)
        .args(args)
        .output()
        .ok()?;
    (output.status.success())
        .then(|| String::from_utf8_lossy(&output.stdout).trim().to_owned())
        .filter(|text| !text.is_empty())
}

/// The repository a path belongs to, or None outside any repository.
pub fn repository(path: &Path) -> Option<Repository> {
    let root = std::path::PathBuf::from(git(path, &["rev-parse", "--show-toplevel"])?);
    let origin = git(&root, &["remote", "get-url", "origin"])?;
    let slug = slug_from_url(&origin)?;
    let branch = git(&root, &["rev-parse", "--abbrev-ref", "HEAD"])
        .filter(|name| name != "HEAD");
    let commit = git(&root, &["rev-parse", "HEAD"]);
    Some(Repository {
        slug,
        branch,
        commit,
        root,
    })
}

/// `owner/name` from every remote form GitHub documents: https, ssh, git://
/// and the `owner/name` shorthand.
pub fn slug_from_url(url: &str) -> Option<String> {
    let trimmed = url.trim().trim_end_matches('/');
    let trimmed = trimmed.strip_suffix(".git").unwrap_or(trimmed);
    // git@github.com:owner/name or ssh://git@github.com/owner/name
    let after_host = trimmed
        .split_once(':')
        .map(|(_, rest)| rest)
        .map(|rest| rest.trim_start_matches('/'))
        .filter(|rest| !rest.is_empty());
    let tail = after_host.or_else(|| {
        // https://github.com/owner/name — take everything after the host.
        trimmed
            .split_once("://")
            .map(|(_, rest)| rest)
            .and_then(|rest| rest.split_once('/'))
            .map(|(_, rest)| rest)
    })?;
    let mut parts = tail.split('/');
    let owner = parts.next()?.trim();
    let name = parts
        .next()?
        .trim()
        .trim_end_matches('/')
        .trim()
        .to_owned();
    if owner.is_empty() || name.is_empty() {
        return None;
    }
    Some(format!("{owner}/{name}"))
}

/// Sub-path of `path` inside the repository, in URL form (`/` separators).
fn relative_url_path(repository: &Repository, path: &Path) -> Option<String> {
    let relative = path.strip_prefix(&repository.root).ok()?;
    let text = relative.to_str()?;
    if text.is_empty() {
        None
    } else {
        Some(
            text.split(std::path::MAIN_SEPARATOR)
                .map(crate::infrastructure::git_url::encode_segment)
                .collect::<Vec<_>>()
                .join("/"),
        )
    }
}

/// The github.com URL for a path: file → blob, directory → tree. Uses the
/// commit SHA when known, so the link is a stable permalink like VS Code's
/// "Copy GitHub permalink"; otherwise it falls back to the branch.
pub fn github_url(repository: &Repository, path: &Path) -> Option<String> {
    let relative = relative_url_path(repository, path);
    let reference = repository
        .commit
        .clone()
        .or_else(|| repository.branch.clone())?;
    let kind = if path.is_dir() { "tree" } else { "blob" };
    Some(match relative {
        Some(relative) => {
            format!("https://github.com/{}/{}", repository.slug, kind) + "/" + &reference + "/" + &relative
        }
        None => format!(
            "https://github.com/{}/{}/{}",
            repository.slug, kind, reference
        ),
    })
}

/// Clone a repository into `destination`, reporting progress lines through
/// `on_line`. Uses `--progress` so git emits its transfer percentages even
/// without a terminal, and fails cleanly when the target already exists.
pub fn clone(url: &str, destination: &Path, mut on_line: impl FnMut(String)) -> std::io::Result<()> {
    use std::io::{BufRead, BufReader};
    use std::process::Stdio;

    if destination.exists()
        && std::fs::read_dir(destination)
            .map(|entries| entries.count() > 0)
            .unwrap_or(false)
    {
        return Err(std::io::Error::new(
            std::io::ErrorKind::AlreadyExists,
            "The destination folder is not empty",
        ));
    }
    let mut child = Command::new("git")
        .args(["clone", "--progress"])
        .arg(url.trim())
        .arg(destination)
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()?;
    if let Some(stderr) = child.stderr.take() {
        for line in BufReader::new(stderr).lines().map_while(Result::ok) {
            on_line(line);
        }
    }
    let status = child.wait()?;
    if status.success() {
        Ok(())
    } else {
        Err(std::io::Error::other("git clone failed"))
    }
}

/// A human summary of the last progress line git emitted.
pub fn progress_summary(line: &str) -> String {
    let stripped = line
        .split('\r')
        .next_back()
        .unwrap_or(line)
        .trim()
        .to_owned();
    if stripped.is_empty() {
        "Working…".to_owned()
    } else {
        stripped
    }
}

#[cfg(test)]
#[path = "../../tests/infrastructure/git.rs"]
mod tests;