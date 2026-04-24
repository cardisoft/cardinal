//! Configurable key bindings for the TUI.
//!
//! # Example config
//! ```toml
//! [global]
//! quit   = "q"
//! leader = "ctrl+w"
//!
//! [leader]
//! focus_query   = ["j", "up"]
//! focus_results = ["k", "down"]
//! help          = "?"
//!
//! [query]
//! clear         = ["esc", "ctrl+u"]
//! submit        = "enter"
//! history_older = "up"
//! history_newer = "down"
//!
//! [results]
//! scroll_down   = ["j", "down"]
//! scroll_up     = ["k", "up"]
//! open_details  = "enter"
//! open_editor   = "v"
//! sort_filename = "1"
//! sort_path     = "2"
//! sort_size     = "3"
//! sort_modified = "4"
//! sort_created  = "5"
//! ```

use anyhow::{Context, Result};
use crossterm::event::{KeyCode, KeyModifiers};
use serde::{Deserialize, Deserializer, de};
use std::{fmt, path::Path, str::FromStr};

// ── KeySpec ──────────────────────────────────────────────────────────────────

/// A single parsed key: a [`KeyCode`] plus a [`KeyModifiers`] bitmask.
///
/// Accepted string syntax: `[modifier+…+]key`
/// where modifier is `ctrl`, `shift`, or `alt`, and key is a named key
/// (`enter`, `esc`, `up`, `down`, `left`, `right`, `home`, `end`,
/// `backspace`, `delete`, `tab`, `space`, `f1`–`f12`) or a single character.
///
/// Examples: `"q"`, `"ctrl+w"`, `"shift+enter"`, `"f5"`
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KeySpec {
    pub code: KeyCode,
    pub mods: KeyModifiers,
}

impl KeySpec {
    pub fn new(code: KeyCode, mods: KeyModifiers) -> Self {
        Self { code, mods }
    }

    pub fn plain(code: KeyCode) -> Self {
        Self {
            code,
            mods: KeyModifiers::NONE,
        }
    }

    pub fn ctrl(ch: char) -> Self {
        Self {
            code: KeyCode::Char(ch),
            mods: KeyModifiers::CONTROL,
        }
    }

    pub fn plain_char(ch: char) -> Self {
        Self::plain(KeyCode::Char(ch))
    }

    /// Returns `true` if this spec matches the given code/modifiers pair.
    pub fn matches(self, code: KeyCode, mods: KeyModifiers) -> bool {
        self.code == code && self.mods == mods
    }
}

impl FromStr for KeySpec {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim();
        let parts: Vec<&str> = s.split('+').collect();
        if parts.is_empty() {
            return Err("empty key spec".to_string());
        }

        let key_str = parts.last().unwrap();
        let modifier_parts = &parts[..parts.len() - 1];

        let mut mods = KeyModifiers::NONE;
        for m in modifier_parts {
            match m.to_lowercase().as_str() {
                "ctrl" | "control" => mods |= KeyModifiers::CONTROL,
                "shift" => mods |= KeyModifiers::SHIFT,
                "alt" | "meta" => mods |= KeyModifiers::ALT,
                "" => {}
                other => return Err(format!("unknown modifier: {other}")),
            }
        }

        let code = match key_str.to_lowercase().as_str() {
            "enter" | "return" => KeyCode::Enter,
            "esc" | "escape" => KeyCode::Esc,
            "backspace" | "bs" => KeyCode::Backspace,
            "delete" | "del" => KeyCode::Delete,
            "left" => KeyCode::Left,
            "right" => KeyCode::Right,
            "up" => KeyCode::Up,
            "down" => KeyCode::Down,
            "home" => KeyCode::Home,
            "end" => KeyCode::End,
            "pageup" | "pgup" | "page_up" => KeyCode::PageUp,
            "pagedown" | "pgdn" | "page_down" => KeyCode::PageDown,
            "tab" => KeyCode::Tab,
            "space" => KeyCode::Char(' '),
            f if f.starts_with('f') && f.len() > 1 => {
                let n: u8 = f[1..]
                    .parse()
                    .map_err(|_| format!("unknown key: {key_str}"))?;
                KeyCode::F(n)
            }
            c if c.chars().count() == 1 => KeyCode::Char(c.chars().next().unwrap()),
            other => return Err(format!("unknown key: {other}")),
        };

        Ok(KeySpec { code, mods })
    }
}

impl<'de> Deserialize<'de> for KeySpec {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        s.parse::<KeySpec>().map_err(de::Error::custom)
    }
}

// ── Key list deserialization (string OR array) ────────────────────────────────

fn deser_key_list<'de, D>(deserializer: D) -> Result<Vec<KeySpec>, D::Error>
where
    D: Deserializer<'de>,
{
    struct Visitor;

    impl<'de> de::Visitor<'de> for Visitor {
        type Value = Vec<KeySpec>;

        fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
            write!(f, "a key string or list of key strings")
        }

        fn visit_str<E: de::Error>(self, v: &str) -> Result<Self::Value, E> {
            Ok(vec![v.parse::<KeySpec>().map_err(E::custom)?])
        }

        fn visit_seq<A: de::SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
            let mut out = Vec::new();
            while let Some(s) = seq.next_element::<String>()? {
                out.push(s.parse::<KeySpec>().map_err(de::Error::custom)?);
            }
            Ok(out)
        }
    }

    deserializer.deserialize_any(Visitor)
}

// ── Default helpers ───────────────────────────────────────────────────────────

fn k(s: &str) -> KeySpec {
    s.parse().expect("hard-coded default key spec is valid")
}

fn ks(specs: &[&str]) -> Vec<KeySpec> {
    specs.iter().map(|s| k(s)).collect()
}

// Global
fn default_quit() -> Vec<KeySpec> {
    ks(&["q"])
}
fn default_leader() -> Vec<KeySpec> {
    ks(&["ctrl+w"])
}

// Leader
fn default_focus_query() -> Vec<KeySpec> {
    ks(&["j", "up"])
}
fn default_focus_results() -> Vec<KeySpec> {
    ks(&["k", "down"])
}
fn default_help() -> Vec<KeySpec> {
    ks(&["?"])
}

// Query
fn default_query_clear() -> Vec<KeySpec> {
    ks(&["esc", "ctrl+u"])
}
fn default_query_submit() -> Vec<KeySpec> {
    ks(&["enter", "down"])
}
fn default_history_older() -> Vec<KeySpec> {
    ks(&["up"])
}
fn default_history_newer() -> Vec<KeySpec> {
    ks(&["down"])
}
fn default_cursor_left() -> Vec<KeySpec> {
    ks(&["left"])
}
fn default_cursor_right() -> Vec<KeySpec> {
    ks(&["right"])
}
fn default_cursor_home() -> Vec<KeySpec> {
    ks(&["home"])
}
fn default_cursor_end() -> Vec<KeySpec> {
    ks(&["end"])
}

// Results
fn default_scroll_down() -> Vec<KeySpec> {
    ks(&["j", "down"])
}
fn default_scroll_up() -> Vec<KeySpec> {
    ks(&["k", "up"])
}
fn default_open_details() -> Vec<KeySpec> {
    ks(&["enter"])
}
fn default_open_editor() -> Vec<KeySpec> {
    ks(&["v"])
}
fn default_sort_filename() -> Vec<KeySpec> {
    ks(&["1"])
}
fn default_sort_path() -> Vec<KeySpec> {
    ks(&["2"])
}
fn default_sort_size() -> Vec<KeySpec> {
    ks(&["3"])
}
fn default_sort_modified() -> Vec<KeySpec> {
    ks(&["4"])
}
fn default_sort_created() -> Vec<KeySpec> {
    ks(&["5"])
}
fn default_open_item() -> Vec<KeySpec> {
    ks(&["o"])
}
fn default_reveal_in_finder() -> Vec<KeySpec> {
    ks(&["r"])
}
fn default_copy_filename() -> Vec<KeySpec> {
    ks(&["y"])
}
fn default_copy_path() -> Vec<KeySpec> {
    ks(&["c"])
}
fn default_quick_look() -> Vec<KeySpec> {
    ks(&["space"])
}
fn default_focus_out() -> Vec<KeySpec> {
    ks(&["esc"])
}

// ── Keymap ────────────────────────────────────────────────────────────────────

/// Top-level keymap with four sections.
#[derive(Deserialize)]
#[serde(default)]
pub struct Keymap {
    pub global: GlobalKeys,
    pub leader: LeaderKeys,
    pub query: QueryKeys,
    pub results: ResultKeys,
}

/// Global keys active in both focus modes (Results) plus the leader prefix.
#[derive(Deserialize)]
pub struct GlobalKeys {
    /// Quit the TUI (active only when Results has focus and no popup is open).
    #[serde(deserialize_with = "deser_key_list", default = "default_quit")]
    pub quit: Vec<KeySpec>,
    /// Leader prefix key (default `Ctrl+W`).
    #[serde(deserialize_with = "deser_key_list", default = "default_leader")]
    pub leader: Vec<KeySpec>,
}

/// Keys active while waiting for the second key of a leader sequence.
#[derive(Deserialize)]
pub struct LeaderKeys {
    #[serde(deserialize_with = "deser_key_list", default = "default_focus_query")]
    pub focus_query: Vec<KeySpec>,
    #[serde(deserialize_with = "deser_key_list", default = "default_focus_results")]
    pub focus_results: Vec<KeySpec>,
    #[serde(deserialize_with = "deser_key_list", default = "default_help")]
    pub help: Vec<KeySpec>,
}

/// Keys active when the Query box has focus.
#[derive(Deserialize)]
pub struct QueryKeys {
    /// Clear the query (and quit if already empty).
    #[serde(deserialize_with = "deser_key_list", default = "default_query_clear")]
    pub clear: Vec<KeySpec>,
    /// Submit the query and move focus to Results.
    #[serde(deserialize_with = "deser_key_list", default = "default_query_submit")]
    pub submit: Vec<KeySpec>,
    #[serde(deserialize_with = "deser_key_list", default = "default_history_older")]
    pub history_older: Vec<KeySpec>,
    #[serde(deserialize_with = "deser_key_list", default = "default_history_newer")]
    pub history_newer: Vec<KeySpec>,
    #[serde(deserialize_with = "deser_key_list", default = "default_cursor_left")]
    pub cursor_left: Vec<KeySpec>,
    #[serde(deserialize_with = "deser_key_list", default = "default_cursor_right")]
    pub cursor_right: Vec<KeySpec>,
    #[serde(deserialize_with = "deser_key_list", default = "default_cursor_home")]
    pub cursor_home: Vec<KeySpec>,
    #[serde(deserialize_with = "deser_key_list", default = "default_cursor_end")]
    pub cursor_end: Vec<KeySpec>,
}

/// Keys active when the Results table has focus.
#[derive(Deserialize)]
pub struct ResultKeys {
    #[serde(deserialize_with = "deser_key_list", default = "default_scroll_down")]
    pub scroll_down: Vec<KeySpec>,
    #[serde(deserialize_with = "deser_key_list", default = "default_scroll_up")]
    pub scroll_up: Vec<KeySpec>,
    #[serde(deserialize_with = "deser_key_list", default = "default_open_details")]
    pub open_details: Vec<KeySpec>,
    #[serde(deserialize_with = "deser_key_list", default = "default_open_editor")]
    pub open_editor: Vec<KeySpec>,
    #[serde(deserialize_with = "deser_key_list", default = "default_open_item")]
    pub open_item: Vec<KeySpec>,
    #[serde(
        deserialize_with = "deser_key_list",
        default = "default_reveal_in_finder"
    )]
    pub reveal_in_finder: Vec<KeySpec>,
    #[serde(deserialize_with = "deser_key_list", default = "default_copy_filename")]
    pub copy_filename: Vec<KeySpec>,
    #[serde(deserialize_with = "deser_key_list", default = "default_copy_path")]
    pub copy_path: Vec<KeySpec>,
    #[serde(deserialize_with = "deser_key_list", default = "default_quick_look")]
    pub quick_look: Vec<KeySpec>,
    #[serde(deserialize_with = "deser_key_list", default = "default_sort_filename")]
    pub sort_filename: Vec<KeySpec>,
    #[serde(deserialize_with = "deser_key_list", default = "default_sort_path")]
    pub sort_path: Vec<KeySpec>,
    #[serde(deserialize_with = "deser_key_list", default = "default_sort_size")]
    pub sort_size: Vec<KeySpec>,
    #[serde(deserialize_with = "deser_key_list", default = "default_sort_modified")]
    pub sort_modified: Vec<KeySpec>,
    #[serde(deserialize_with = "deser_key_list", default = "default_sort_created")]
    pub sort_created: Vec<KeySpec>,
    #[serde(deserialize_with = "deser_key_list", default = "default_focus_out")]
    pub focus_out: Vec<KeySpec>, // TODO: add default?
}

// ── Default impls ─────────────────────────────────────────────────────────────

impl Default for Keymap {
    fn default() -> Self {
        Self {
            global: GlobalKeys::default(),
            leader: LeaderKeys::default(),
            query: QueryKeys::default(),
            results: ResultKeys::default(),
        }
    }
}

impl Default for GlobalKeys {
    fn default() -> Self {
        Self {
            quit: default_quit(),
            leader: default_leader(),
        }
    }
}

impl Default for LeaderKeys {
    fn default() -> Self {
        Self {
            focus_query: default_focus_query(),
            focus_results: default_focus_results(),
            help: default_help(),
        }
    }
}

impl Default for QueryKeys {
    fn default() -> Self {
        Self {
            clear: default_query_clear(),
            submit: default_query_submit(),
            history_older: default_history_older(),
            history_newer: default_history_newer(),
            cursor_left: default_cursor_left(),
            cursor_right: default_cursor_right(),
            cursor_home: default_cursor_home(),
            cursor_end: default_cursor_end(),
        }
    }
}

impl Default for ResultKeys {
    fn default() -> Self {
        Self {
            scroll_down: default_scroll_down(),
            scroll_up: default_scroll_up(),
            open_details: default_open_details(),
            open_editor: default_open_editor(),
            open_item: default_open_item(),
            reveal_in_finder: default_reveal_in_finder(),
            copy_filename: default_copy_filename(),
            copy_path: default_copy_path(),
            quick_look: default_quick_look(),
            sort_filename: default_sort_filename(),
            sort_path: default_sort_path(),
            sort_size: default_sort_size(),
            sort_modified: default_sort_modified(),
            sort_created: default_sort_created(),
            focus_out: default_focus_out(),
        }
    }
}

// ── Loading ───────────────────────────────────────────────────────────────────

impl Keymap {
    /// Load a keymap from a TOML file, merging with built-in defaults for any
    /// missing sections or keys.  Returns the default keymap if the file does
    /// not exist.
    pub fn load(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let raw = std::fs::read_to_string(path)
            .with_context(|| format!("reading keymap file {}", path.display()))?;
        toml::from_str(&raw).with_context(|| format!("parsing keymap file {}", path.display()))
    }
}

/// Returns `true` if any binding in `specs` matches `(code, mods)`.
pub fn match_key(specs: &[KeySpec], code: KeyCode, mods: KeyModifiers) -> bool {
    specs.iter().any(|s| s.matches(code, mods))
}
