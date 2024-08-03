use std::sync::LazyLock;

use indexmap::IndexMap;
use regex::Regex;

pub fn parse_change_log(changelog: &str) -> Changelog<'_> {
    let (header, changelog) = changelog
        .find("## [Unreleased]")
        .map(|pos| (changelog[0..pos].trim(), &changelog[pos..]))
        .expect("no '## [Unreleased]' found");

    let mut offset = 0;

    let mut foot_links = Vec::new();

    for line in changelog.split_inclusive('\n').rev() {
        if FOOTER_REGEX.is_match(line) {
            let text = {
                let start = memchr::memchr(b'[', line.as_bytes()).unwrap();
                let end = memchr::memchr(b']', line.as_bytes()).unwrap();

                line[start + 1..end].trim()
            };

            let link = {
                let colon = memchr::memchr(b':', line.as_bytes()).unwrap();

                line[colon + 1..].trim()
            };

            foot_links.push(FootLink { text, link });

            offset += line.len();
        } else {
            break;
        }
    }

    let mut changelog = changelog[..changelog.len() - offset].trim();

    let mut releases = IndexMap::new();

    // dbg!(&changelog);

    while let Some((release, offset)) = Release::new(changelog) {
        releases.insert(release.version, release);

        changelog = &changelog[offset..];
    }

    let res = Changelog {
        header,
        releases,
        foot_links,
    };

    // println!("{:?}", res);

    dbg!(&res);

    res
}

// todo: capture
static FOOTER_REGEX: LazyLock<Regex> = LazyLock::new(|| regex::Regex::new(r"\[.*\]:").unwrap());

static RELEASE_TITLE_REGEX: LazyLock<Regex> =
    LazyLock::new(|| regex::Regex::new(r"## \[(.*)\](?: - (.*))?").unwrap());

// todo: capture
static RELEASE_SECTION_TITLE_REGEX: LazyLock<Regex> =
    LazyLock::new(|| regex::Regex::new(r"### .*").unwrap());

static RELEASE_NOTE_REGEX: LazyLock<Regex> =
    LazyLock::new(|| regex::Regex::new(r" - (.*)").unwrap());

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Changelog<'a> {
    pub header: &'a str,
    pub releases: IndexMap<&'a str, Release<'a>>,
    pub foot_links: Vec<FootLink<'a>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FootLink<'a> {
    pub text: &'a str,
    pub link: &'a str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Release<'a> {
    pub version: &'a str,
    pub title: Option<&'a str>,
    pub notes: IndexMap<&'a str, Vec<&'a str>>,
}

impl<'a> Release<'a> {
    fn new(text: &'a str) -> Option<(Self, usize)> {
        dbg!(&text);

        let lines = text.split_inclusive('\n');

        #[derive(Clone, Debug, PartialEq, Eq)]
        enum State<'a> {
            Init,
            Title,
            Section {
                title: &'a str,
                start: usize,
                end: usize,
            },
        }

        let mut state = State::Init;

        let mut version = None;

        let mut title = None;

        let mut notes = IndexMap::new();

        let mut pos = 0;

        for line in lines {
            if RELEASE_TITLE_REGEX.is_match(line) {
                match state {
                    State::Init => {
                        let res = RELEASE_TITLE_REGEX.captures(line).unwrap();

                        version = res.get(1).map(|s| s.as_str());
                        title = res.get(2).map(|s| s.as_str());

                        state = State::Title;
                    }
                    State::Title => break,
                    State::Section { title, start, end } => {
                        notes.insert(title, text[start..end].trim());
                        break;
                    }
                }
            } else if RELEASE_SECTION_TITLE_REGEX.is_match(line) {
                match state {
                    State::Init => panic!("release section {} found before a release title", line),
                    State::Title => {}
                    State::Section { title, start, end } => {
                        notes.insert(title, text[start..end].trim());
                    }
                }

                let title = { line["### ".len()..].trim() };
                state = State::Section {
                    title,
                    start: pos + line.len(),
                    end: pos + line.len(),
                }
            } else if let State::Section {
                title: _,
                start: _,
                end,
            } = &mut state
            {
                *end += line.len();
            } else {
                // panic!("not in a section({:?}): {}", state, line);
                // todo: find out what to do in this situation
            }

            pos += line.len();
        }

        match state {
            State::Init => return None,
            State::Title => {}
            State::Section { title, start, end } => {
                notes.insert(title, text[start..end].trim());
            }
        }

        let release = Release {
            version: version.unwrap(),
            title,
            notes,
        };

        Some((release, pos))
    }
}

#[cfg(test)]
mod test {
    use std::{fs::File, io::Read};

    use crate::change_log::{
        FOOTER_REGEX, RELEASE_NOTE_REGEX, RELEASE_SECTION_TITLE_REGEX, RELEASE_TITLE_REGEX,
    };

    use super::parse_change_log;

    #[test]
    fn test_changelog1() {
        let mut file = File::open("tests/changelogs/CHANGELOG1.md").unwrap();

        let mut changelog = String::new();

        file.read_to_string(&mut changelog).unwrap();

        parse_change_log(&changelog);
    }

    #[test]
    fn test_changelog2() {
        let mut file = File::open("tests/changelogs/CHANGELOG2.md").unwrap();

        let mut changelog = String::new();

        file.read_to_string(&mut changelog).unwrap();

        parse_change_log(&changelog);
    }

    #[test]
    fn test_changelog3() {
        let mut file = File::open("tests/changelogs/CHANGELOG3.md").unwrap();

        let mut changelog = String::new();

        file.read_to_string(&mut changelog).unwrap();

        parse_change_log(&changelog);
    }

    #[test]
    fn test_regex() {
        assert!(FOOTER_REGEX.is_match("[hello]:"));
        assert!(!FOOTER_REGEX.is_match("[hello]"));
        assert!(!FOOTER_REGEX.is_match("[hello:"));
        assert!(!FOOTER_REGEX.is_match("hello]:"));
    }

    #[test]
    fn test_regex_release() {
        assert!(RELEASE_TITLE_REGEX.is_match("## [2024.7.30]"));
        assert!(!RELEASE_TITLE_REGEX.is_match("# [2024.7.30]"));
        assert!(RELEASE_TITLE_REGEX.is_match("## [Unreleased]"));
        assert!(RELEASE_TITLE_REGEX.is_match("## [2024.7] - 2024-07-24"));
        assert!(!RELEASE_TITLE_REGEX.is_match("##[2024.7] - 2024-07-24"));
        assert!(!RELEASE_TITLE_REGEX.is_match("# [2024.7] - 2024-07-24"));
        assert!(RELEASE_TITLE_REGEX.is_match("## [2024.7] - a"));

        let res = RELEASE_TITLE_REGEX
            .captures("## [2024.7] - 2024-07-24")
            .unwrap();

        assert_eq!(res.get(1).unwrap().as_str(), "2024.7");
        assert_eq!(res.get(2).unwrap().as_str(), "2024-07-24");

        // should be false
        assert!(RELEASE_TITLE_REGEX.is_match("## [2024.7] -2024-07-24"));
        assert!(RELEASE_TITLE_REGEX.is_match("## [2024.7] -  "));
    }

    #[test]
    fn test_regex_release_section() {
        assert!(RELEASE_SECTION_TITLE_REGEX.is_match("### Added"));
        assert!(!RELEASE_SECTION_TITLE_REGEX.is_match("## Added"));
        assert!(!RELEASE_SECTION_TITLE_REGEX.is_match("###Added"));
        assert!(RELEASE_SECTION_TITLE_REGEX.is_match("### [Added]"));
    }

    #[test]
    fn release_note_regex() {
        assert!(RELEASE_NOTE_REGEX.is_match("- fix: hello"));
        assert!(!RELEASE_SECTION_TITLE_REGEX.is_match("## Added"));
        assert!(!RELEASE_SECTION_TITLE_REGEX.is_match("###Added"));
        assert!(RELEASE_SECTION_TITLE_REGEX.is_match("### [Added]"));
    }
}
