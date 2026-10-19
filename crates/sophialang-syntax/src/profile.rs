//! Rendering profiles (SPEC-005): named sets of settings that decide how
//! code is displayed. The settings live in the database as key/value rows;
//! this module turns them into a [`Profile`] and back. See `profile.md`.

use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Blocks {
    End,
    Braces,
    Indent,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Profile {
    pub name: String,
    /// `function`, `fn` or `def`.
    pub kw_function: String,
    pub blocks: Blocks,
    /// `::` (Julia) or `:` (Rust, Python).
    pub annotation: String,
    pub unicode: bool,
    pub index_base: u8,
    /// Whether text in this profile parses back (SPEC-005 §5, rule 4).
    pub editable: bool,
    /// The core term itself, for debugging (read-only).
    pub core: bool,
    /// Every setting, including ones this version does not interpret.
    pub settings: BTreeMap<String, String>,
}

pub const BUILT_IN: [&str; 4] = ["julia", "rust", "python", "core"];

impl Profile {
    pub fn built_in(name: &str) -> Option<Profile> {
        let kv: &[(&str, &str)] = match name {
            "julia" => &[
                ("kw.function", "function"),
                ("blocks", "end"),
                ("type.annotation", "::"),
                ("operators", "ascii"),
                ("index.base", "1"),
            ],
            "rust" => &[
                ("kw.function", "fn"),
                ("blocks", "braces"),
                ("type.annotation", ":"),
                ("operators", "ascii"),
                ("index.base", "0"),
            ],
            "python" => &[
                ("kw.function", "def"),
                ("blocks", "indent"),
                ("type.annotation", ":"),
                ("operators", "ascii"),
                ("index.base", "0"),
            ],
            "core" => &[("view", "core")],
            _ => return None,
        };
        let settings = kv.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect();
        Some(Profile::from_settings(name, &settings))
    }

    pub fn julia() -> Profile {
        Profile::built_in("julia").unwrap()
    }

    /// Builds a profile from settings rows. Unknown or invalid values fall
    /// back to the Julia defaults (SR-27, SR-29).
    pub fn from_settings(name: &str, settings: &BTreeMap<String, String>) -> Profile {
        let get = |k: &str| settings.get(k).map(String::as_str);
        let blocks = match get("blocks") {
            Some("braces") => Blocks::Braces,
            Some("indent") => Blocks::Indent,
            _ => Blocks::End,
        };
        let kw_function = match get("kw.function") {
            Some(k @ ("fn" | "def" | "function")) => k.to_string(),
            _ => "function".into(),
        };
        let annotation = match get("type.annotation") {
            Some(":") => ":".to_string(),
            _ => "::".to_string(),
        };
        let core = get("view") == Some("core");
        Profile {
            name: name.into(),
            kw_function,
            editable: !core && blocks != Blocks::Indent,
            blocks,
            annotation,
            unicode: get("operators") == Some("unicode"),
            index_base: if get("index.base") == Some("0") { 0 } else { 1 },
            core,
            settings: settings.clone(),
        }
    }

    /// The annotation as printed between a name and its type.
    pub fn ann(&self) -> &'static str {
        if self.annotation == ":" { ": " } else { "::" }
    }

    pub fn pi_arrow(&self) -> &'static str {
        if self.unicode { "→" } else { "-->" }
    }
}

/// Parses the text form of a profile (the `sophia:Settings` view, SPEC-004
/// §4): `key = value` lines; `#` starts a comment; `based_on = name` names
/// the profile it inherits from.
pub fn parse_settings(text: &str) -> Result<BTreeMap<String, String>, String> {
    let mut out = BTreeMap::new();
    for (n, line) in text.lines().enumerate() {
        let line = line.split('#').next().unwrap_or("").trim();
        if line.is_empty() || line.starts_with('[') {
            continue;
        }
        let (k, v) = line.split_once('=').ok_or_else(|| format!("line {}: expected `key = value`", n + 1))?;
        let k = k.trim();
        let v = v.trim().trim_matches('"');
        if k.is_empty() || k.chars().any(|c| c.is_whitespace()) {
            return Err(format!("line {}: bad key", n + 1));
        }
        out.insert(k.to_string(), v.to_string());
    }
    Ok(out)
}

/// The text form of settings rows.
pub fn print_settings(profile_name: &str, settings: &BTreeMap<String, String>) -> String {
    let mut s = format!("# rendering profile `{profile_name}` (stored in the database; this text is a view)\n");
    for (k, v) in settings {
        s.push_str(&format!("{k} = \"{v}\"\n"));
    }
    s
}
