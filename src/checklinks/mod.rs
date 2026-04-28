//! Link validation for markdown file trees.
//!
//! [`check_dir`] walks a directory recursively (respecting `.gitignore`),
//! parses every `.md` file with `pulldown-cmark`, extracts all links, and
//! validates them:
//!
//! - Same-file anchors (`#heading`) — checked against the file's headings.
//! - Cross-file links (`./other.md`) — checked that the target file exists.
//! - Cross-file anchors (`./other.md#section`) — file AND anchor checked.
//! - External (`http(s)://`) and `mailto:`/`ftp://` — skipped silently
//!   (unless `CheckOpts::check_external` is set, which is currently a stub).

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::time::Instant;

use ignore::WalkBuilder;
use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};

use crate::markdown::heading_to_anchor;

// ── Public surface ────────────────────────────────────────────────────────────

/// Options controlling link-validation behaviour.
#[derive(Debug, Clone, Default)]
pub struct CheckOpts {
    /// When `true`, `http(s)://` links are validated via HEAD requests.
    ///
    /// Currently stubbed: enabling this flag prints a notice and continues
    /// with internal-only validation. No HTTP client dependency is added.
    pub check_external: bool,
}

/// A single broken link found during validation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BrokenLink {
    /// 1-based source line where the link appears.
    pub line: u32,
    /// Human-readable description of why the link is broken.
    pub reason: String,
    /// The raw link target as it appears in the markdown source.
    pub raw_target: String,
}

/// Validation results for one `.md` file.
#[derive(Debug, Clone)]
pub struct FileReport {
    /// Path of the file that was validated (relative to the scan root).
    pub path: PathBuf,
    /// All broken links found in this file.
    pub broken: Vec<BrokenLink>,
}

/// Aggregated report returned by [`check_dir`].
#[derive(Debug)]
pub struct CheckReport {
    /// Per-file results — only files with at least one broken link are included.
    pub files: Vec<FileReport>,
    /// Total number of `.md` files that were scanned.
    pub files_scanned: usize,
    /// Total number of broken links across all files.
    pub broken_count: usize,
    /// Wall-clock time for the full scan.
    pub elapsed: std::time::Duration,
}

impl CheckReport {
    /// Returns `true` when no broken links were found.
    pub fn is_clean(&self) -> bool {
        self.broken_count == 0
    }

    /// Print a human-readable summary to stdout.
    pub fn print(&self, root: &Path) {
        println!("Checking links in {} ...\n", root.display());

        for file_report in &self.files {
            println!("{}:", file_report.path.display());
            for broken in &file_report.broken {
                println!(
                    "  line {}: {}  [{}]",
                    broken.line, broken.reason, broken.raw_target
                );
            }
            println!();
        }

        let secs = self.elapsed.as_secs_f64();
        if self.broken_count == 0 {
            println!(
                "All links OK. Scanned {} file(s) in {:.2}s.",
                self.files_scanned, secs
            );
        } else {
            let file_count = self.files.len();
            println!(
                "{} broken link(s) across {} file(s) ({} .md files scanned in {:.2}s).",
                self.broken_count, file_count, self.files_scanned, secs
            );
        }
    }
}

/// Walk `root` recursively, validate every `.md` file, and return an
/// aggregated [`CheckReport`].
///
/// # Arguments
///
/// * `root` - Directory to scan (recursively).
/// * `opts` - Validation options (e.g. whether to check external links).
///
/// # Panics
///
/// Does not panic; individual file-read errors are silently skipped.
pub fn check_dir(root: &Path, opts: &CheckOpts) -> CheckReport {
    let started = Instant::now();

    if opts.check_external {
        eprintln!("note: external link checking is not yet implemented; skipping HTTP(S) links.");
    }

    // ── Phase 1: collect all markdown files and parse their headings ──────────
    // We need the heading sets before validating cross-file anchor links, so we
    // do a first pass to build an index.
    let md_paths = collect_md_files(root);
    let files_scanned = md_paths.len();

    // anchor_index maps each absolute path → set of anchor slugs present in
    // the file. We populate this in a single pass so cross-file anchor checks
    // are O(1) lookups.
    let anchor_index: HashMap<PathBuf, HashSet<String>> = md_paths
        .iter()
        .map(|p| {
            let anchors = parse_anchors_from_file(p);
            (p.clone(), anchors)
        })
        .collect();

    // ── Phase 2: validate links in every file ─────────────────────────────────
    let mut file_reports: Vec<FileReport> = Vec::new();
    let mut total_broken = 0usize;

    for abs_path in &md_paths {
        let content = match std::fs::read_to_string(abs_path) {
            Ok(c) => c,
            // Unreadable file — skip silently; this is uncommon (permissions,
            // binary files named .md, etc.) and not a link-validation concern.
            Err(_) => continue,
        };

        let broken = validate_links(abs_path, &content, &anchor_index);

        if !broken.is_empty() {
            total_broken += broken.len();
            let rel_path = abs_path
                .strip_prefix(root)
                .unwrap_or(abs_path)
                .to_path_buf();
            file_reports.push(FileReport {
                path: rel_path,
                broken,
            });
        }
    }

    // Sort by file path for stable, predictable output.
    file_reports.sort_by(|a, b| a.path.cmp(&b.path));

    CheckReport {
        files: file_reports,
        files_scanned,
        broken_count: total_broken,
        elapsed: started.elapsed(),
    }
}

// ── Internal helpers ──────────────────────────────────────────────────────────

/// Collect all `.md` files under `root` using the `ignore` crate's walker,
/// which honours `.gitignore` rules and skips hidden directories.
fn collect_md_files(root: &Path) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    for entry in WalkBuilder::new(root).build().flatten() {
        let path = entry.into_path();
        if path.is_file() && path.extension().is_some_and(|e| e == "md") {
            paths.push(path);
        }
    }
    // Sort for deterministic iteration order (WalkBuilder may yield in any order).
    paths.sort();
    paths
}

/// Parse all headings from `path` and return their anchor slugs.
///
/// Uses `pulldown-cmark` — headings inside code fences are correctly excluded.
fn parse_anchors_from_file(path: &Path) -> HashSet<String> {
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return HashSet::new(),
    };
    parse_anchors(&content)
}

/// Extract anchor slugs from all headings in a markdown string.
///
/// Duplicate heading texts produce duplicate slugs; callers that need
/// disambiguation must handle it themselves (GitHub disambiguates with `-1`,
/// `-2`, etc., but for link validation we accept any occurrence as valid).
fn parse_anchors(content: &str) -> HashSet<String> {
    let opts = Options::ENABLE_TABLES
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_TASKLISTS
        | Options::ENABLE_MATH;

    let parser = Parser::new_ext(content, opts);
    let mut anchors = HashSet::new();
    let mut in_heading = false;
    let mut heading_text = String::new();

    for event in parser {
        match event {
            Event::Start(Tag::Heading { .. }) => {
                in_heading = true;
                heading_text.clear();
            }
            // Collapse the guard into the match arm to satisfy clippy::collapsible_match.
            Event::End(TagEnd::Heading(_)) if in_heading => {
                anchors.insert(heading_to_anchor(&heading_text));
                in_heading = false;
                heading_text.clear();
            }
            Event::End(TagEnd::Heading(_)) => {}
            Event::Text(text) | Event::Code(text) if in_heading => {
                // pulldown-cmark yields inline code spans as `Event::Code`
                // even inside headings. GitHub includes inline code text in
                // the slug, so we do the same.
                heading_text.push_str(&text);
            }
            _ => {}
        }
    }
    anchors
}

/// The category of a link target, used to decide how to validate it.
#[derive(Debug)]
enum LinkKind {
    /// `#fragment` — same-file anchor check.
    SameFileAnchor(String),
    /// `./other.md` or `path/other.md` — cross-file, no anchor.
    CrossFile(PathBuf),
    /// `./other.md#fragment` — cross-file with anchor.
    CrossFileAnchor(PathBuf, String),
    /// `http(s)://...` — external; only checked when `opts.check_external`.
    External,
    /// `mailto:`, `ftp://`, etc. — silently ignored.
    Ignored,
}

/// Classify a raw URL string into a [`LinkKind`].
fn classify_url(url: &str, file_dir: &Path) -> LinkKind {
    if let Some(fragment) = url.strip_prefix('#') {
        // Same-file anchor.
        return LinkKind::SameFileAnchor(fragment.to_string());
    }

    if url.starts_with("http://") || url.starts_with("https://") {
        return LinkKind::External;
    }

    // Any other scheme (mailto:, ftp:, tel:, etc.) — skip silently.
    if url.contains("://") || url.starts_with("mailto:") {
        return LinkKind::Ignored;
    }

    // Relative path — may contain a `#fragment`.
    // Split on the first `#` to separate path from fragment.
    let (path_part, fragment) = match url.find('#') {
        Some(idx) => (&url[..idx], Some(&url[idx + 1..])),
        None => (url, None),
    };

    // Resolve relative path against the directory that contains this file.
    let target = file_dir.join(path_part);

    match fragment {
        Some(frag) => LinkKind::CrossFileAnchor(target, frag.to_string()),
        None => LinkKind::CrossFile(target),
    }
}

/// A raw link extracted from a markdown source, with its approximate line.
struct RawLink {
    /// The URL/target exactly as written in the markdown source.
    url: String,
    /// 1-based line number derived from pulldown-cmark's byte-offset span.
    line: u32,
}

/// Extract all links from a markdown string, together with their source lines.
///
/// pulldown-cmark resolves reference-style links (`[text][label]` +
/// `[label]: url`) automatically, so no special handling is required here.
fn extract_links(content: &str) -> Vec<RawLink> {
    let opts = Options::ENABLE_TABLES
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_TASKLISTS
        | Options::ENABLE_MATH;

    // Build a byte-offset → line-number map so we can convert the span start
    // offsets that pulldown-cmark gives us into 1-based line numbers.
    let line_starts = build_line_starts(content);

    // `into_offset_iter()` wraps each event with its byte range so we can
    // derive source line numbers without a separate byte-scan pass.
    let parser = Parser::new_ext(content, opts).into_offset_iter();

    let mut links = Vec::new();
    let mut current_link: Option<(String, u32)> = None;

    for (event, range) in parser {
        match event {
            Event::Start(Tag::Link { dest_url, .. }) => {
                let line = byte_offset_to_line(range.start, &line_starts);
                current_link = Some((dest_url.into_string(), line));
            }
            Event::End(TagEnd::Link) => {
                if let Some((url, line)) = current_link.take() {
                    links.push(RawLink { url, line });
                }
            }
            _ => {}
        }
    }
    links
}

/// Build a sorted list of byte offsets where each line starts.
///
/// `line_starts[i]` is the byte offset of the first character on line `i`
/// (0-indexed). The vector always starts with `0`.
fn build_line_starts(content: &str) -> Vec<usize> {
    let mut starts = vec![0usize];
    for (i, ch) in content.char_indices() {
        if ch == '\n' {
            starts.push(i + 1);
        }
    }
    starts
}

/// Convert a byte offset to a 1-based line number using the pre-built line
/// start table.
fn byte_offset_to_line(offset: usize, line_starts: &[usize]) -> u32 {
    // Binary search for the largest start ≤ offset; the index is the 0-based
    // line number. Add 1 for 1-based output.
    let idx = line_starts.partition_point(|&s| s <= offset);
    // `partition_point` returns the index *after* the last match, so subtract
    // 1 to get the line that contains `offset`.
    idx.saturating_sub(1) as u32 + 1
}

/// Validate all links in `content` (from file `abs_path`) and return any
/// broken ones.
///
/// # Arguments
///
/// * `abs_path`     - Absolute path of the file being validated.
/// * `content`      - Raw markdown source of that file.
/// * `root`         - Scan root (used for display purposes only).
/// * `anchor_index` - Pre-built map of `absolute_path → anchor slug set`.
fn validate_links(
    abs_path: &Path,
    content: &str,
    anchor_index: &HashMap<PathBuf, HashSet<String>>,
) -> Vec<BrokenLink> {
    let file_dir = abs_path.parent().unwrap_or(Path::new("."));
    let self_anchors = parse_anchors(content);
    let raw_links = extract_links(content);
    let mut broken = Vec::new();

    for raw in raw_links {
        match classify_url(&raw.url, file_dir) {
            LinkKind::SameFileAnchor(anchor) => {
                if !self_anchors.contains(&anchor) {
                    broken.push(BrokenLink {
                        line: raw.line,
                        // Show the raw target in the reason so it's grep-friendly.
                        reason: format!("broken anchor {}", &raw.url),
                        raw_target: raw.url,
                    });
                }
            }

            LinkKind::CrossFile(target) => {
                // Canonicalise the path so that `..` components are resolved
                // without requiring the file to exist (which `canonicalize`
                // would need). We use `normalize_path` below.
                let resolved = normalize_path(&target);
                if !resolved.exists() {
                    broken.push(BrokenLink {
                        line: raw.line,
                        reason: format!("missing file {}", &raw.url),
                        raw_target: raw.url,
                    });
                }
            }

            LinkKind::CrossFileAnchor(target, anchor) => {
                let resolved = normalize_path(&target);
                if !resolved.exists() {
                    broken.push(BrokenLink {
                        line: raw.line,
                        reason: format!("missing file {}", &raw.url),
                        raw_target: raw.url,
                    });
                } else {
                    // File exists — check the anchor within it.
                    let anchors = anchor_index.get(&resolved).cloned().unwrap_or_else(|| {
                        // The file exists but wasn't in the index (e.g. not a
                        // tracked .md file or added after the index was built).
                        // Parse it on demand.
                        parse_anchors_from_file(&resolved)
                    });
                    if !anchors.contains(&anchor) {
                        broken.push(BrokenLink {
                            line: raw.line,
                            reason: format!("broken cross-file anchor {}", &raw.url),
                            raw_target: raw.url,
                        });
                    }
                }
            }

            // External links are skipped (stub for --check-external).
            // Non-http schemes (mailto:, ftp:, etc.) are silently ignored.
            LinkKind::External | LinkKind::Ignored => {}
        }
    }

    broken
}

/// Resolve `..` and `.` components in a path without requiring it to exist
/// on disk (unlike `std::fs::canonicalize`).
///
/// This is a best-effort normalisation: it handles the common `./foo/../bar`
/// patterns that appear in markdown cross-file links.
fn normalize_path(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for component in path.components() {
        match component {
            std::path::Component::ParentDir => {
                // Pop the last real component if possible.
                if !out.pop() {
                    out.push(component);
                }
            }
            std::path::Component::CurDir => {
                // Skip `.` components.
            }
            other => out.push(other),
        }
    }
    out
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    /// Helper: write files into a temp directory and return (TempDir, PathBuf
    /// of the temp dir root). `TempDir` must be kept alive for the test.
    fn make_temp_dir(files: &[(&str, &str)]) -> (TempDir, PathBuf) {
        let dir = tempfile::tempdir().expect("failed to create tempdir");
        let root = dir.path().to_path_buf();
        for (name, content) in files {
            let path = root.join(name);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).expect("failed to create subdir");
            }
            fs::write(&path, content).expect("failed to write test file");
        }
        (dir, root)
    }

    // ── parse_anchors ─────────────────────────────────────────────────────────

    #[test]
    fn parse_anchors_extracts_heading_slugs() {
        let content = "# Hello World\n\n## API v2.0\n\nsome text\n";
        let anchors = parse_anchors(content);
        assert!(anchors.contains("hello-world"), "expected 'hello-world'");
        assert!(anchors.contains("api-v20"), "expected 'api-v20'");
    }

    // ── validate_links / unit-level ───────────────────────────────────────────

    #[test]
    fn valid_internal_anchor_passes() {
        let (_dir, root) = make_temp_dir(&[("doc.md", "# Title\n\n[link](#title)\n")]);
        let report = check_dir(&root, &CheckOpts::default());
        assert_eq!(report.broken_count, 0, "expected no broken links");
    }

    #[test]
    fn broken_internal_anchor_reported() {
        let (_dir, root) = make_temp_dir(&[("doc.md", "# Title\n\n[link](#nonexistent)\n")]);
        let report = check_dir(&root, &CheckOpts::default());
        assert_eq!(report.broken_count, 1, "expected exactly one broken link");
        assert_eq!(report.files[0].broken[0].raw_target, "#nonexistent");
    }

    #[test]
    fn valid_cross_file_link_passes() {
        let (_dir, root) = make_temp_dir(&[("a.md", "[link](./b.md)\n"), ("b.md", "# B file\n")]);
        let report = check_dir(&root, &CheckOpts::default());
        assert_eq!(report.broken_count, 0, "expected no broken links");
    }

    #[test]
    fn missing_file_reported() {
        let (_dir, root) = make_temp_dir(&[("a.md", "[link](./nonexistent.md)\n")]);
        let report = check_dir(&root, &CheckOpts::default());
        assert_eq!(report.broken_count, 1, "expected exactly one broken link");
        assert_eq!(report.files[0].broken[0].raw_target, "./nonexistent.md");
    }

    #[test]
    fn cross_file_with_valid_anchor_passes() {
        let (_dir, root) = make_temp_dir(&[
            ("a.md", "[link](./b.md#real-section)\n"),
            ("b.md", "# Real Section\n\nsome content.\n"),
        ]);
        let report = check_dir(&root, &CheckOpts::default());
        assert_eq!(report.broken_count, 0, "expected no broken links");
    }

    #[test]
    fn cross_file_with_bad_anchor_reported() {
        let (_dir, root) = make_temp_dir(&[
            ("a.md", "[link](./b.md#fake)\n"),
            ("b.md", "# Real Section\n\nsome content.\n"),
        ]);
        let report = check_dir(&root, &CheckOpts::default());
        assert_eq!(report.broken_count, 1, "expected exactly one broken link");
        assert!(
            report.files[0].broken[0].raw_target.contains("#fake"),
            "raw_target should contain #fake"
        );
    }

    #[test]
    fn external_link_skipped_silently_when_check_external_off() {
        let (_dir, root) = make_temp_dir(&[("doc.md", "[link](https://example.com)\n")]);
        let report = check_dir(
            &root,
            &CheckOpts {
                check_external: false,
            },
        );
        assert_eq!(report.broken_count, 0, "external links must be skipped");
    }

    // ── helpers ───────────────────────────────────────────────────────────────

    #[test]
    fn normalize_path_resolves_parent_components() {
        let p = PathBuf::from("/tmp/docs/../other.md");
        assert_eq!(normalize_path(&p), PathBuf::from("/tmp/other.md"));
    }

    #[test]
    fn byte_offset_to_line_maps_correctly() {
        // "abc\ndef\n" — line 1 starts at 0, line 2 starts at 4.
        let content = "abc\ndef\n";
        let starts = build_line_starts(content);
        assert_eq!(byte_offset_to_line(0, &starts), 1);
        assert_eq!(byte_offset_to_line(3, &starts), 1); // the '\n' itself
        assert_eq!(byte_offset_to_line(4, &starts), 2); // 'd'
        assert_eq!(byte_offset_to_line(7, &starts), 2);
    }
}
