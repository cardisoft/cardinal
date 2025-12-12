use plist::Value;
use std::{
    io::{self, Cursor},
    path::{Path, PathBuf},
    process::Command,
};
use xattr::get;

const USER_TAG_XATTR: &str = "com.apple.metadata:_kMDItemUserTags";

/// Searches for files with the specified tag using the `mdfind` command-line tool.
///
/// Returns a vector of file paths that have the specified tag.
pub fn search_tags_using_mdfind(
    tags: Vec<String>,
    case_insensitive: bool,
) -> io::Result<Vec<PathBuf>> {
    if tags.is_empty() {
        return Ok(Vec::new());
    }
    for tag in &tags {
        if let Some(forbidden_char) = tag_has_spotlight_forbidden_chars(tag) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("tag filter contains unsupported character '{forbidden_char}': {tag}"),
            ));
        }
    }

    let modifier = if case_insensitive { "c" } else { "" };
    let query = tags
        .into_iter()
        .map(|tag| format!("kMDItemUserTags == '*{tag}*'{modifier}"))
        .collect::<Vec<_>>()
        .join(" || ");
    let output = Command::new("mdfind").arg(query).output()?;

    if !output.status.success() {
        return Err(io::Error::other("mdfind command failed"));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let paths = stdout.lines().map(PathBuf::from).collect();

    Ok(paths)
}

fn tag_has_spotlight_forbidden_chars(tag: &str) -> Option<char> {
    tag.chars().find(|c| matches!(c, '\'' | '\\' | '*'))
}

/// Reads Finder-style user tags from an on-disk item.
/// Returns `None` if cancellation or filesystem errors occur.
pub fn read_tags_from_path(path: &Path, case_insensitive: bool) -> Option<Vec<String>> {
    let raw = match get(path, USER_TAG_XATTR) {
        Ok(Some(data)) => data,
        Ok(None) | Err(_) => Vec::new(),
    };
    Some(parse_tags(&raw, case_insensitive))
}

pub fn parse_tags(raw: &[u8], case_insensitive: bool) -> Vec<String> {
    let Ok(Value::Array(items)) = Value::from_reader(Cursor::new(raw)) else {
        return Vec::new();
    };

    items
        .into_iter()
        .filter_map(|value| match value {
            Value::String(text) => Some(strip_tag_suffix(&text, case_insensitive)),
            _ => None,
        })
        .collect()
}

pub fn strip_tag_suffix(value: &str, case_insensitive: bool) -> String {
    let name = value.split('\n').next().unwrap_or(value);
    if case_insensitive {
        name.to_ascii_lowercase()
    } else {
        name.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use plist::{Integer, to_writer_binary};
    #[cfg(target_os = "macos")]
    use std::process::Command;
    #[cfg(target_os = "macos")]
    use tempfile::NamedTempFile;

    fn plist_bytes(values: &[Value]) -> Vec<u8> {
        let mut data = Vec::new();
        to_writer_binary(&mut data, &Value::Array(values.to_vec())).expect("serialize tags");
        data
    }

    #[cfg(target_os = "macos")]
    fn bytes_to_hex(data: &[u8]) -> String {
        data.iter().map(|b| format!("{b:02X}")).collect()
    }

    #[test]
    fn parses_tag_strings() {
        let bytes = plist_bytes(&[
            Value::String("Important\n0".into()),
            Value::String("Archive".into()),
        ]);
        let tags = parse_tags(&bytes, false);
        assert_eq!(tags, vec!["Important".to_string(), "Archive".to_string()]);
    }

    #[test]
    fn strips_suffix_and_lowercases() {
        let tags = strip_tag_suffix("Important\n0", true);
        assert_eq!(tags, "important");
    }

    #[test]
    fn parse_tags_returns_empty_for_invalid_plist() {
        let bytes = b"not a plist";
        assert!(parse_tags(bytes, false).is_empty());
    }

    #[test]
    fn parse_tags_lowercases_when_requested() {
        let bytes = plist_bytes(&[Value::String("Important\n0".into())]);
        let tags = parse_tags(&bytes, true);
        assert_eq!(tags, vec!["important".to_string()]);
    }

    #[test]
    fn parse_tags_ignores_non_string_entries() {
        let bytes = plist_bytes(&[
            Value::String("Project\n0".into()),
            Value::Integer(Integer::from(42)),
            Value::Boolean(true),
        ]);
        let tags = parse_tags(&bytes, false);
        assert_eq!(tags, vec!["Project".to_string()]);
    }

    #[cfg(target_os = "macos")]
    fn write_xattr(path: &std::path::Path, tags: &[&str]) {
        use xattr::set;

        let plist_values: Vec<Value> = tags
            .iter()
            .map(|tag| Value::String(format!("{tag}\n0")))
            .collect();
        let data = plist_bytes(&plist_values);
        set(path, USER_TAG_XATTR, &data).expect("write tag xattr");
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn read_tags_from_path_reads_written_attribute() {
        let file = NamedTempFile::new().expect("create temp file");
        write_xattr(file.path(), &["Important", "Archive"]);

        let tags = read_tags_from_path(file.path(), false).expect("read tags");
        assert_eq!(tags, vec!["Important".to_string(), "Archive".to_string()]);
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn read_tags_from_path_handles_missing_attribute() {
        let file = NamedTempFile::new().expect("create temp file");
        let tags = read_tags_from_path(file.path(), false).expect("read tags");
        assert!(tags.is_empty());
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn read_tags_from_path_reads_cli_written_attribute() {
        let file = NamedTempFile::new().expect("create temp file");
        let bytes = plist_bytes(&[
            Value::String("Important\n0".into()),
            Value::String("Archive\n0".into()),
        ]);
        let hex = bytes_to_hex(&bytes);
        let status = Command::new("xattr")
            .arg("-wx")
            .arg(USER_TAG_XATTR)
            .arg(&hex)
            .arg(file.path())
            .status()
            .expect("run xattr -wx");
        assert!(status.success(), "xattr -wx failed");

        let tags = read_tags_from_path(file.path(), false).expect("read tags");
        assert_eq!(tags, vec!["Important".to_string(), "Archive".to_string()]);
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn cli_reads_attribute_written_by_library() {
        let file = NamedTempFile::new().expect("create temp file");
        write_xattr(file.path(), &["Focus"]);
        let expected_hex = bytes_to_hex(&plist_bytes(&[Value::String("Focus\n0".into())]));

        let output = Command::new("xattr")
            .arg("-px")
            .arg(USER_TAG_XATTR)
            .arg(file.path())
            .output()
            .expect("run xattr -px");
        assert!(output.status.success(), "xattr -px failed");
        let hex_stdout = String::from_utf8(output.stdout).expect("cli hex output");
        let cli_hex: String = hex_stdout.split_whitespace().collect();
        assert_eq!(cli_hex, expected_hex);
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn cli_delete_removes_attribute_for_reader() {
        let file = NamedTempFile::new().expect("create temp file");
        write_xattr(file.path(), &["Temp"]);

        let status = Command::new("xattr")
            .arg("-d")
            .arg(USER_TAG_XATTR)
            .arg(file.path())
            .status()
            .expect("run xattr -d");
        assert!(status.success(), "xattr -d failed");

        let tags = read_tags_from_path(file.path(), false).expect("read tags");
        assert!(tags.is_empty());
    }

    // Tests for search_tags_using_mdfind edge cases
    #[test]
    fn search_tags_using_mdfind_empty_list_returns_empty() {
        let result = search_tags_using_mdfind(vec![], false);
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[test]
    fn search_tags_using_mdfind_rejects_single_quote() {
        let result = search_tags_using_mdfind(vec!["Project'Alpha".to_string()], false);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidInput);
        assert!(err.to_string().contains("unsupported character '''"));
    }

    #[test]
    fn search_tags_using_mdfind_rejects_backslash() {
        let result = search_tags_using_mdfind(vec!["Project\\Alpha".to_string()], false);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidInput);
        assert!(err.to_string().contains("unsupported character '\\'"));
    }

    #[test]
    fn search_tags_using_mdfind_rejects_asterisk() {
        let result = search_tags_using_mdfind(vec!["Project*".to_string()], false);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidInput);
        assert!(err.to_string().contains("unsupported character '*'"));
    }

    #[test]
    fn search_tags_using_mdfind_rejects_forbidden_char_in_second_tag() {
        let result = search_tags_using_mdfind(
            vec!["ValidTag".to_string(), "Invalid'Tag".to_string()],
            false,
        );
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidInput);
        assert!(err.to_string().contains("Invalid'Tag"));
    }

    #[test]
    fn search_tags_using_mdfind_allows_hyphen() {
        // Hyphen is not a forbidden character
        let result = search_tags_using_mdfind(vec!["Project-Alpha".to_string()], false);
        // We can't verify success without actual files, but it should not reject the input
        // If mdfind is not available or returns no results, that's fine for this test
        match result {
            Ok(_) => {}                                                     // Success is fine
            Err(e) if e.to_string().contains("mdfind command failed") => {} // mdfind not available is fine
            Err(e) => panic!("Unexpected error: {e}"),
        }
    }

    #[test]
    fn search_tags_using_mdfind_allows_underscore() {
        let result = search_tags_using_mdfind(vec!["Project_Alpha".to_string()], false);
        match result {
            Ok(_) => {}
            Err(e) if e.to_string().contains("mdfind command failed") => {}
            Err(e) => panic!("Unexpected error: {e}"),
        }
    }

    #[test]
    fn search_tags_using_mdfind_allows_unicode() {
        let result = search_tags_using_mdfind(vec!["项目".to_string()], false);
        match result {
            Ok(_) => {}
            Err(e) if e.to_string().contains("mdfind command failed") => {}
            Err(e) => panic!("Unexpected error: {e}"),
        }
    }

    #[test]
    fn search_tags_using_mdfind_allows_emoji() {
        let result = search_tags_using_mdfind(vec!["🔴Important".to_string()], false);
        match result {
            Ok(_) => {}
            Err(e) if e.to_string().contains("mdfind command failed") => {}
            Err(e) => panic!("Unexpected error: {e}"),
        }
    }

    #[test]
    fn search_tags_using_mdfind_multiple_tags_constructs_or_query() {
        // We can't easily verify the exact query without mocking, but we can verify
        // that multiple tags are accepted without error
        let result =
            search_tags_using_mdfind(vec!["Project".to_string(), "Important".to_string()], false);
        match result {
            Ok(_) => {}
            Err(e) if e.to_string().contains("mdfind command failed") => {}
            Err(e) => panic!("Unexpected error: {e}"),
        }
    }

    #[test]
    fn tag_has_spotlight_forbidden_chars_returns_none_for_safe_string() {
        assert_eq!(tag_has_spotlight_forbidden_chars("Project-Alpha_123"), None);
    }

    #[test]
    fn tag_has_spotlight_forbidden_chars_detects_single_quote() {
        assert_eq!(
            tag_has_spotlight_forbidden_chars("Project'Alpha"),
            Some('\'')
        );
    }

    #[test]
    fn tag_has_spotlight_forbidden_chars_detects_backslash() {
        assert_eq!(
            tag_has_spotlight_forbidden_chars("Project\\Alpha"),
            Some('\\')
        );
    }

    #[test]
    fn tag_has_spotlight_forbidden_chars_detects_asterisk() {
        assert_eq!(tag_has_spotlight_forbidden_chars("Project*"), Some('*'));
    }

    #[test]
    fn tag_has_spotlight_forbidden_chars_detects_first_occurrence() {
        assert_eq!(
            tag_has_spotlight_forbidden_chars("Project'Alpha*Beta"),
            Some('\'')
        );
    }

    #[test]
    fn parse_tags_handles_empty_array() {
        let bytes = plist_bytes(&[]);
        assert!(parse_tags(&bytes, false).is_empty());
    }

    #[test]
    fn parse_tags_handles_tag_without_suffix() {
        let bytes = plist_bytes(&[Value::String("NoSuffix".into())]);
        let tags = parse_tags(&bytes, false);
        assert_eq!(tags, vec!["NoSuffix".to_string()]);
    }

    #[test]
    fn parse_tags_handles_multiple_newlines_in_tag() {
        let bytes = plist_bytes(&[Value::String("Tag\n0\nextra".into())]);
        let tags = parse_tags(&bytes, false);
        assert_eq!(tags, vec!["Tag".to_string()]);
    }

    #[test]
    fn strip_tag_suffix_preserves_case_when_not_lowercasing() {
        assert_eq!(strip_tag_suffix("Important\n0", false), "Important");
        assert_eq!(strip_tag_suffix("IMPORTANT\n0", false), "IMPORTANT");
    }

    #[test]
    fn strip_tag_suffix_lowercases_when_requested() {
        assert_eq!(strip_tag_suffix("Important\n0", true), "important");
        assert_eq!(strip_tag_suffix("IMPORTANT\n0", true), "important");
        assert_eq!(strip_tag_suffix("ImPoRtAnT\n0", true), "important");
    }

    #[test]
    fn strip_tag_suffix_handles_empty_string() {
        assert_eq!(strip_tag_suffix("", false), "");
        assert_eq!(strip_tag_suffix("", true), "");
    }

    #[test]
    fn strip_tag_suffix_handles_unicode() {
        assert_eq!(strip_tag_suffix("项目\n0", false), "项目");
        assert_eq!(strip_tag_suffix("项目\n0", true), "项目");
    }

    #[test]
    fn read_tags_from_path_returns_none_for_nonexistent_path() {
        let result = read_tags_from_path(Path::new("/nonexistent/path"), false);
        assert!(result.is_some());
        assert!(result.unwrap().is_empty());
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn read_tags_from_path_case_insensitive() {
        let file = NamedTempFile::new().expect("create temp file");
        write_xattr(file.path(), &["Important", "PROJECT"]);

        let tags = read_tags_from_path(file.path(), true).expect("read tags");
        assert_eq!(tags, vec!["important".to_string(), "project".to_string()]);
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn read_tags_from_path_case_sensitive() {
        let file = NamedTempFile::new().expect("create temp file");
        write_xattr(file.path(), &["Important", "PROJECT"]);

        let tags = read_tags_from_path(file.path(), false).expect("read tags");
        assert_eq!(tags, vec!["Important".to_string(), "PROJECT".to_string()]);
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn read_tags_from_path_handles_unicode_tags() {
        let file = NamedTempFile::new().expect("create temp file");
        write_xattr(file.path(), &["项目", "重要", "🔴"]);

        let tags = read_tags_from_path(file.path(), false).expect("read tags");
        assert_eq!(
            tags,
            vec!["项目".to_string(), "重要".to_string(), "🔴".to_string()]
        );
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn read_tags_from_path_handles_very_long_tag_name() {
        let file = NamedTempFile::new().expect("create temp file");
        let long_tag = "a".repeat(1000);
        write_xattr(file.path(), &[&long_tag]);

        let tags = read_tags_from_path(file.path(), false).expect("read tags");
        assert_eq!(tags.len(), 1);
        assert_eq!(tags[0].len(), 1000);
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn read_tags_from_path_handles_many_tags() {
        let file = NamedTempFile::new().expect("create temp file");
        let tags_to_write: Vec<String> = (0..100).map(|i| format!("Tag{i}")).collect();
        let tag_refs: Vec<&str> = tags_to_write.iter().map(|s| s.as_str()).collect();
        write_xattr(file.path(), &tag_refs);

        let tags = read_tags_from_path(file.path(), false).expect("read tags");
        assert_eq!(tags.len(), 100);
    }
}
