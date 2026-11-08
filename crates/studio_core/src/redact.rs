//! The user's home folder written as `~` in text a member posts: the
//! window's log lines and a repro bundle's journal.

/// `line` with the user's home folder written as `~`, because findings carry absolute
/// paths and the log is posted where the cup's members can read it (ST-RUN-07). No home
/// known (`None`) and an empty one both leave the line as it is. On Windows, where paths
/// ignore letter case and take either slash, the home folder is matched however it is
/// written (a PES folder typed `c:/users/...`).
pub fn redact_home(line: &str, home: Option<&str>) -> String {
    match home {
        Some(home) if !home.is_empty() && cfg!(windows) => redact_any_spelling(line, home),
        Some(home) if !home.is_empty() => line.replace(home, "~"),
        _ => line.to_string(),
    }
}

/// `line` with every run of characters that spells `home` in any letter case, with `/`
/// and `\` alike, written as `~`; the rest of the line as written. A separator may
/// also be spelled `\\`, the escape a TOML or JSON basic string writes for one.
fn redact_any_spelling(line: &str, home: &str) -> String {
    let same = |a: char, b: char| {
        matches!((a, b), ('/' | '\\', '/' | '\\')) || a.to_lowercase().eq(b.to_lowercase())
    };
    // The byte length of `rest`'s start when it spells `home`.
    let spelled_len = |rest: &str| {
        let mut chars = rest.chars();
        let mut len = 0;
        for wanted in home.chars() {
            let found = chars.next().filter(|found| same(wanted, *found))?;
            len += found.len_utf8();
            // A `\\` pair is one escaped separator: its second backslash
            // belongs to the spelling too.
            if found == '\\' && chars.as_str().starts_with('\\') {
                len += chars
                    .next()
                    .expect("the pair's second backslash")
                    .len_utf8();
            }
        }
        Some(len)
    };
    let mut redacted = String::with_capacity(line.len());
    let mut rest = line;
    while let Some(next) = rest.chars().next() {
        let taken = match spelled_len(rest) {
            Some(len) => {
                redacted.push('~');
                len
            }
            None => {
                redacted.push(next);
                next.len_utf8()
            }
        };
        rest = &rest[taken..];
    }
    redacted
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Windows paths ignore letter case and take either slash, so the home
    /// folder written another way is redacted too, the rest of the line as
    /// written.
    #[cfg(windows)]
    // ST-RUN-07
    #[test]
    fn redact_home_matches_any_spelling_of_the_home_folder_on_windows() {
        let home = Some("C:\\Users\\Alice");
        assert_eq!(
            redact_home("PES folder C:/Users/Alice/Games/PES2017 is missing", home),
            "PES folder ~/Games/PES2017 is missing"
        );
        assert_eq!(
            redact_home("opened c:\\users\\alice\\x", home),
            "opened ~\\x"
        );
    }

    /// A `\` inside a TOML or JSON basic string is written `\\`: the
    /// escaped pair spells the one separator, so a home folder in a
    /// double-quoted value is redacted too.
    #[cfg(windows)]
    // ST-RUN-07
    #[test]
    fn redact_home_matches_the_escaped_separator_of_a_quoted_string() {
        let home = Some("C:\\Users\\Alice");
        assert_eq!(
            redact_home("dir = \"C:\\\\Users\\\\Alice\\\\Games\"", home),
            "dir = \"~\\\\Games\""
        );
    }

    /// Off Windows paths are letter-case exact: the home folder written
    /// exactly is redacted.
    #[cfg(not(windows))]
    // ST-RUN-07
    #[test]
    fn redact_home_replaces_the_home_folder_as_written() {
        assert_eq!(
            redact_home("opened /home/alice/x", Some("/home/alice")),
            "opened ~/x"
        );
    }

    #[test]
    fn redact_home_without_a_home_leaves_the_line_as_it_is() {
        let line = "opened C:\\Users\\member\\a.bin";
        assert_eq!(redact_home(line, None), line);
        assert_eq!(redact_home(line, Some("")), line);
    }
}
