use super::*;

fn repository(path: &str) -> Repository {
    crate::infrastructure::git::repository(Path::new(path)).expect("the virial repo is a repository")
}

#[test]
fn slug_parsing_covers_every_github_remote_form() {
    let cases = [
        ("https://github.com/DeamonDev888/virial-gpui.git", "DeamonDev888/virial-gpui"),
        ("https://github.com/DeamonDev888/virial-gpui", "DeamonDev888/virial-gpui"),
        ("git@github.com:DeamonDev888/virial-gpui.git", "DeamonDev888/virial-gpui"),
        ("ssh://git@github.com/DeamonDev888/virial-gpui.git", "DeamonDev888/virial-gpui"),
        ("git://github.com/DeamonDev888/virial-gpui.git", "DeamonDev888/virial-gpui"),
        ("DeamonDev888/virial-gpui", "DeamonDev888/virial-gpui"),
    ];
    for (url, expected) in cases {
        assert_eq!(slug_from_url(url).as_deref(), Some(expected), "{url}");
    }
}

#[test]
fn slug_parsing_rejects_foreign_and_incomplete_urls() {
    for url in [
        "https://gitlab.com/owner/repo.git",
        "https://bitbucket.org/owner/repo",
        "https://github.com/owner",
        "https://github.com/",
        "not a url",
        "",
    ] {
        assert_eq!(slug_from_url(url), None, "{url}");
    }
}

#[test]
fn the_virial_checkout_maps_to_github_urls() {
    // The test runs inside the repository, so the virial checkout itself is
    // the fixture: no setup, always real.
    let repo = repository(env!("CARGO_MANIFEST_DIR"));
    assert_eq!(repo.slug, "DeamonDev888/virial-gpui");
    assert!(repo.commit.is_some(), "a checked-out repo has a commit");
    let root_url = github_url(&repo, &repo.root).unwrap();
    let commit = repo.commit.clone().unwrap();
    assert_eq!(
        root_url,
        format!("https://github.com/DeamonDev888/virial-gpui/tree/{commit}")
    );
    let manifest = repo.root.join("Cargo.toml");
    let file_url = github_url(&repo, &manifest).unwrap();
    assert_eq!(
        file_url,
        format!("https://github.com/DeamonDev888/virial-gpui/blob/{commit}/Cargo.toml")
    );
}

#[test]
fn a_path_outside_the_repository_has_no_url() {
    let repo = repository(env!("CARGO_MANIFEST_DIR"));
    let outside = std::env::temp_dir().join("virial-gh-outside-test");
    assert_eq!(github_url(&repo, &outside), None);
}

#[test]
fn a_missing_file_still_gets_a_blob_url() {
    // The URL builder must not touch the filesystem for the target itself:
    // deleted files get permalinks too, which is the point of a permalink.
    let repo = repository(env!("CARGO_MANIFEST_DIR"));
    let missing = repo.root.join("src").join("does-not-exist.rs");
    let url = github_url(&repo, &missing).unwrap();
    assert!(url.contains("/blob/"), "{url}");
}

#[test]
fn unicode_and_special_names_are_percent_encoded() {
    let repo = repository(env!("CARGO_MANIFEST_DIR"));
    let file = repo.root.join("café #1?.rs");
    let url = github_url(&repo, &file).unwrap();
    assert!(url.contains("caf%C3%A9%20%231%3F.rs"), "{url}");
    assert!(!url.contains(' '), "{url}");
}

#[test]
fn clone_refuses_a_non_empty_destination() {
    let directory = tempfile::tempdir().unwrap();
    let existing = directory.path().join("full");
    std::fs::create_dir(&existing).unwrap();
    std::fs::write(existing.join("keep.txt"), b"data").unwrap();
    let error = clone(
        "https://github.com/DeamonDev888/nonexistent-xyz",
        &existing,
        |_| {},
    )
    .unwrap_err();
    assert_eq!(error.kind(), std::io::ErrorKind::AlreadyExists);
    assert_eq!(
        std::fs::read_to_string(existing.join("keep.txt")).unwrap(),
        "data",
        "the folder is untouched"
    );
}

#[test]
fn clone_fails_cleanly_on_an_unreachable_url() {
    let directory = tempfile::tempdir().unwrap();
    let destination = directory.path().join("clone");
    let result = clone(
        "https://github.invalid/owner/repo.git",
        &destination,
        |_| {},
    );
    assert!(result.is_err());
    assert!(
        !destination.exists() || std::fs::read_dir(&destination).unwrap().count() == 0,
        "no partial checkout is left behind"
    );
}

#[test]
fn progress_summary_prefers_the_last_carriage_return_frame() {
    let frame = "Receiving objects:  42% (21/50), 1.2 MiB | 300 KiB/s\rReceiving objects:  99% (50/50)";
    assert_eq!(
        progress_summary(frame),
        "Receiving objects:  99% (50/50)"
    );
    assert_eq!(progress_summary(""), "Working…");
}