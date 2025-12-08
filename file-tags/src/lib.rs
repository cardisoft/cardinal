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
pub fn search_tags_using_mdfind(tag: &str, case_insensitive: bool) -> io::Result<Vec<PathBuf>> {
    if tag_has_spotlight_forbidden_chars(tag) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("tag filter contains characters unsupported: {tag}"),
        ));
    }

    let modifier = if case_insensitive { "c" } else { "" };
    let query = format!("kMDItemUserTags == '*{tag}*'{modifier}");
    let output = Command::new("mdfind").arg(query).output()?;

    if !output.status.success() {
        return Err(io::Error::other("mdfind command failed"));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let paths = stdout.lines().map(PathBuf::from).collect();

    Ok(paths)
}

fn tag_has_spotlight_forbidden_chars(tag: &str) -> bool {
    tag.chars().any(|c| matches!(c, '\'' | '\\' | '*'))
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
}
