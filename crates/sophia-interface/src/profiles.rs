//! Rendering profiles stored in the database (SPEC-005 §2): rows of the
//! store's meta table, never a file. See `profiles.md`.
//!
//! Keys:
//! - `profile/<name>/<setting>` = value;  `profile/<name>/based_on` = parent
//! - `reader/<reader>/<scope>` = profile name, where scope is `default`,
//!   `origin:<lang>` or `namespace:<path>`

use std::collections::BTreeMap;

use sophia_store::{Backend, Store, StoreError};
use sophialang_syntax::Profile;
use sophialang_syntax::profile::BUILT_IN;

type Result<T> = std::result::Result<T, StoreError>;

fn utf8(v: Vec<u8>) -> String {
    String::from_utf8_lossy(&v).into_owned()
}

/// Writes the built-in profiles if they are missing.
pub fn ensure_built_ins<B: Backend>(store: &Store<B>) -> Result<()> {
    for name in BUILT_IN {
        if store.meta_prefix(&format!("profile/{name}/"))?.is_empty() {
            let p = Profile::built_in(name).expect("built-in");
            let rows: Vec<(String, Option<String>)> =
                p.settings.iter().map(|(k, v)| (format!("profile/{name}/{k}"), Some(v.clone()))).collect();
            store.set_settings(&format!("profile/{name}"), &rows)?;
        }
    }
    Ok(())
}

/// A profile's own settings rows (without inheritance).
pub fn own_settings<B: Backend>(store: &Store<B>, name: &str) -> Result<BTreeMap<String, String>> {
    let prefix = format!("profile/{name}/");
    Ok(store
        .meta_prefix(&prefix)?
        .into_iter()
        .map(|(k, v)| (k[prefix.len()..].to_string(), utf8(v)))
        .collect())
}

/// A profile's settings with inheritance through `based_on` resolved.
pub fn settings<B: Backend>(store: &Store<B>, name: &str) -> Result<BTreeMap<String, String>> {
    let mut chain = vec![name.to_string()];
    let mut merged = BTreeMap::new();
    let mut at = name.to_string();
    for _ in 0..16 {
        let own = own_settings(store, &at)?;
        let parent = own.get("based_on").cloned();
        for (k, v) in own {
            merged.entry(k).or_insert(v);
        }
        match parent {
            Some(p) if !chain.contains(&p) => {
                chain.push(p.clone());
                at = p;
            }
            _ => break,
        }
    }
    Ok(merged)
}

pub fn profile<B: Backend>(store: &Store<B>, name: &str) -> Result<Profile> {
    let s = settings(store, name)?;
    Ok(if s.is_empty() { Profile::julia() } else { Profile::from_settings(name, &s) })
}

/// The profile a reader uses for a definition in namespace `ns` that came
/// from language `lang`: `namespace:` (longest prefix) beats `origin:`,
/// which beats `default`; Julia when nothing is set (SR-27).
pub fn profile_name_for<B: Backend>(store: &Store<B>, reader: &str, ns: &str, lang: &str) -> Result<String> {
    let rows: BTreeMap<String, String> = store
        .meta_prefix(&format!("reader/{reader}/"))?
        .into_iter()
        .map(|(k, v)| (k.split_once(&format!("reader/{reader}/")).map(|x| x.1.to_string()).unwrap_or_default(), utf8(v)))
        .collect();
    let mut best: Option<(usize, &String)> = None;
    for (scope, prof) in &rows {
        if let Some(path) = scope.strip_prefix("namespace:") {
            let matches = ns == path || ns.starts_with(&format!("{path}.")) || path.is_empty();
            if matches && best.is_none_or(|(l, _)| path.len() > l) {
                best = Some((path.len(), prof));
            }
        }
    }
    if let Some((_, p)) = best {
        return Ok(p.clone());
    }
    if let Some(p) = rows.get(&format!("origin:{lang}")) {
        return Ok(p.clone());
    }
    Ok(rows.get("default").cloned().unwrap_or_else(|| "julia".into()))
}

/// Sets which profile a reader uses for a scope.
pub fn set_reader_profile<B: Backend>(store: &Store<B>, reader: &str, scope: &str, profile: &str) -> Result<u64> {
    store.set_settings(&format!("reader/{reader}"), &[(format!("reader/{reader}/{scope}"), Some(profile.into()))])
}

/// Replaces a profile's own settings with `rows` (removing keys not given).
pub fn replace_settings<B: Backend>(store: &Store<B>, name: &str, rows: &BTreeMap<String, String>) -> Result<u64> {
    let old = own_settings(store, name)?;
    let mut changes: Vec<(String, Option<String>)> =
        old.keys().filter(|k| !rows.contains_key(*k)).map(|k| (format!("profile/{name}/{k}"), None)).collect();
    changes.extend(rows.iter().map(|(k, v)| (format!("profile/{name}/{k}"), Some(v.clone()))));
    store.set_settings(&format!("profile/{name}"), &changes)
}
