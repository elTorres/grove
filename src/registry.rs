//! The grammar registry — Phase 2 spine.
//!
//! Resolves a language id (or a file path) to its wasm grammar + tags query,
//! loaded at runtime. The local-directory stub here (`registry/<lang>/` with
//! `grammar.wasm`, `tags.scm`, `manifest.json`) stands in for the future hosted
//! WASM registry; nothing above this module knows grammars aren't compiled in.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};

use anyhow::{Context, Result};
use serde::Deserialize;
use sha2::{Digest, Sha256};

/// Language-specific node kinds that drive the structural niceties (parent
/// grouping, enclosing-function for callers, go-to-def). Data, not code — so a
/// new language is fully supported by dropping a manifest, with no recompile.
#[derive(Deserialize, Clone, Default)]
pub struct Profile {
    /// Node kinds that are function/method definitions.
    #[serde(default)]
    pub function_kinds: Vec<String>,
    /// Container node kinds paired with the field that names them, e.g.
    /// `["impl_item", "type"]` or `["class_definition", "name"]`.
    #[serde(default)]
    pub containers: Vec<(String, String)>,
    /// Node kinds whose text is a usable identifier (for go-to-def).
    #[serde(default)]
    pub identifier_kinds: Vec<String>,
}

#[derive(Deserialize, Clone)]
pub struct Manifest {
    pub name: String,
    pub version: String,
    pub extensions: Vec<String>,
    #[serde(default)]
    pub profile: Profile,
}

/// A resolved grammar artifact — enough to load and extract. Cheap to clone
/// (the heavy wasm bytes and query text are shared via `Arc`).
#[derive(Clone)]
pub struct Grammar {
    pub name: String,
    pub version: String,
    pub wasm: Arc<Vec<u8>>,
    pub tags_query: Arc<String>,
    pub profile: Arc<Profile>,
}

impl Grammar {
    /// sha256 of the wasm bytes — the lockfile's integrity field.
    pub fn wasm_sha256(&self) -> String {
        let mut h = Sha256::new();
        h.update(self.wasm.as_slice());
        format!("sha256:{:x}", h.finalize())
    }
}

/// The OS-native global cache location for grammars:
/// `~/.cache/grove/grammars` (Linux), `~/Library/Caches/grove/grammars` (macOS),
/// `%LOCALAPPDATA%\grove\grammars` (Windows). Grammars are a cache — reconstructible
/// from the hosted registry and content-addressed by `grove.lock`.
pub fn cache_root() -> Option<PathBuf> {
    dirs::cache_dir().map(|c| c.join("grove").join("grammars"))
}

/// Dev fallback: the registry shipped in the source tree (only exists in a checkout).
fn dev_root() -> PathBuf {
    PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/registry"))
}

/// A candidate registry root with where it came from, for diagnostics.
pub struct RootCandidate {
    pub source: &'static str,
    pub path: PathBuf,
    pub exists: bool,
}

/// The ordered search path, first existing wins. Surfaced by `grove registry`.
pub fn search_path() -> Vec<RootCandidate> {
    let mut out = Vec::new();
    let mut add = |source, path: PathBuf| {
        let exists = path.is_dir();
        out.push(RootCandidate { source, path, exists });
    };
    if let Ok(p) = std::env::var("GROVE_REGISTRY") {
        add("GROVE_REGISTRY", PathBuf::from(p));
    }
    if let Ok(cwd) = std::env::current_dir() {
        for dir in cwd.ancestors() {
            let cand = dir.join(".grove").join("grammars");
            if cand.is_dir() {
                add("project (.grove/grammars)", cand);
                break;
            }
        }
    }
    if let Some(c) = cache_root() {
        add("user cache", c);
    }
    add("dev (source tree)", dev_root());
    out
}

/// Resolve the registry root by precedence: an explicit `GROVE_REGISTRY` always
/// wins; otherwise the first existing of project-vendored → user cache → dev tree;
/// otherwise the canonical user-cache path (so errors point at the right home).
fn registry_root() -> PathBuf {
    if let Ok(p) = std::env::var("GROVE_REGISTRY") {
        return PathBuf::from(p);
    }
    for cand in search_path() {
        if cand.exists {
            return cand.path;
        }
    }
    cache_root().unwrap_or_else(dev_root)
}

/// Index of available languages, read once from the registry manifests.
struct Index {
    root: PathBuf,
    by_name: HashMap<String, Manifest>,
    by_ext: HashMap<String, String>, // extension -> language name
}

fn index() -> &'static Index {
    static INDEX: OnceLock<Index> = OnceLock::new();
    INDEX.get_or_init(|| {
        let root = registry_root();
        let mut by_name = HashMap::new();
        let mut by_ext = HashMap::new();
        if let Ok(entries) = std::fs::read_dir(&root) {
            for e in entries.flatten() {
                let mpath = e.path().join("manifest.json");
                let Ok(text) = std::fs::read_to_string(&mpath) else {
                    continue;
                };
                let Ok(m) = serde_json::from_str::<Manifest>(&text) else {
                    continue;
                };
                for ext in &m.extensions {
                    by_ext.insert(ext.clone(), m.name.clone());
                }
                by_name.insert(m.name.clone(), m);
            }
        }
        Index { root, by_name, by_ext }
    })
}

fn cache() -> &'static Mutex<HashMap<String, Grammar>> {
    static CACHE: OnceLock<Mutex<HashMap<String, Grammar>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Resolve a grammar by language id, reading (and caching) its artifacts.
pub fn resolve(lang: &str) -> Result<Grammar> {
    if let Some(g) = cache().lock().unwrap().get(lang) {
        return Ok(g.clone());
    }
    let idx = index();
    let manifest = idx.by_name.get(lang).with_context(|| {
        format!(
            "language `{lang}` is not in the registry ({}). Available: {}",
            idx.root.display(),
            available().join(", ")
        )
    })?;
    let dir = idx.root.join(lang);
    let wasm = std::fs::read(dir.join("grammar.wasm"))
        .with_context(|| format!("reading grammar.wasm for `{lang}`"))?;
    let tags = std::fs::read_to_string(dir.join("tags.scm"))
        .with_context(|| format!("reading tags.scm for `{lang}`"))?;
    let grammar = Grammar {
        name: manifest.name.clone(),
        version: manifest.version.clone(),
        wasm: Arc::new(wasm),
        tags_query: Arc::new(tags),
        profile: Arc::new(manifest.profile.clone()),
    };
    cache()
        .lock()
        .unwrap()
        .insert(lang.to_string(), grammar.clone());
    Ok(grammar)
}

/// The language id for a file path, by extension.
pub fn lang_for_path(path: &Path) -> Option<&'static str> {
    let ext = path.extension()?.to_str()?;
    index().by_ext.get(ext).map(String::as_str)
}

/// Resolve the grammar for a file path.
pub fn for_path(path: &Path) -> Result<Grammar> {
    let lang = lang_for_path(path).with_context(|| {
        format!(
            "no registered grammar for `{}` (extensions: {})",
            path.display(),
            extensions().join(", ")
        )
    })?;
    resolve(lang)
}

/// True if the path has a registered extension.
pub fn is_source(path: &Path) -> bool {
    lang_for_path(path).is_some()
}

/// The registry root actually in use this run.
pub fn root() -> &'static Path {
    &index().root
}

/// All available language names.
pub fn available() -> Vec<String> {
    let mut v: Vec<String> = index().by_name.keys().cloned().collect();
    v.sort();
    v
}

/// All registered extensions.
pub fn extensions() -> Vec<String> {
    let mut v: Vec<String> = index().by_ext.keys().cloned().collect();
    v.sort();
    v
}

/// The manifests, for `grove languages`.
pub fn manifests() -> Vec<Manifest> {
    let mut v: Vec<Manifest> = index().by_name.values().cloned().collect();
    v.sort_by(|a, b| a.name.cmp(&b.name));
    v
}

/// Write a lockfile pinning every registry grammar's version + wasm hash.
pub fn write_lock(path: &Path) -> Result<usize> {
    write_lock_for(&available(), path)
}

/// Write a lockfile pinning the given languages' version + wasm hash.
/// Deterministic (sorted) so it is diff-friendly and commit-able.
pub fn write_lock_for(langs: &[String], path: &Path) -> Result<usize> {
    let mut names: Vec<String> = langs.to_vec();
    names.sort();
    names.dedup();
    let mut grammars = Vec::new();
    for name in &names {
        let g = resolve(name)?;
        grammars.push(serde_json::json!({
            "name": g.name,
            "version": g.version,
            "wasm": g.wasm_sha256(),
        }));
    }
    let doc = serde_json::json!({ "version": 1, "grammars": grammars });
    std::fs::write(path, format!("{}\n", serde_json::to_string_pretty(&doc)?))
        .with_context(|| format!("writing {}", path.display()))?;
    Ok(grammars.len())
}
