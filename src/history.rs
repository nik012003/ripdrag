use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use gtk::glib;

use crate::ARGS;

const MAX_ENTRIES: usize = 1000;

fn history_dir() -> PathBuf {
    glib::user_data_dir().join("ripdrag")
}

fn history_file() -> PathBuf {
    history_dir().join("history")
}

/// Appends the paths to the history file, keeping only the newest entries.
pub fn record<P: AsRef<Path>>(paths: impl IntoIterator<Item = P>) {
    if ARGS.get().unwrap().no_history {
        return;
    }
    let new_entries: Vec<String> = paths
        .into_iter()
        .filter_map(|path| fs::canonicalize(path).ok())
        .filter_map(|path| path.to_str().map(str::to_owned))
        .collect();
    if new_entries.is_empty() {
        return;
    }

    let file = history_file();
    let old_entries = fs::read_to_string(&file).unwrap_or_default();
    let entries: Vec<&str> = old_entries
        .lines()
        .chain(new_entries.iter().map(String::as_str))
        .collect();
    let kept = &entries[entries.len().saturating_sub(MAX_ENTRIES)..];

    let _ = fs::create_dir_all(history_dir());
    if let Err(err) = fs::write(&file, kept.join("\n") + "\n") {
        eprintln!("{}: {}", file.display(), err);
    }
}

/// Returns the most recent distinct paths that still exist and are not in PATH, newest first.
pub fn load() -> Vec<PathBuf> {
    let entries = fs::read_to_string(history_file()).unwrap_or_default();
    let mut seen: HashSet<PathBuf> = ARGS
        .get()
        .unwrap()
        .paths
        .iter()
        .filter_map(|path| fs::canonicalize(path).ok())
        .collect();
    entries
        .lines()
        .rev()
        .map(PathBuf::from)
        .filter(|path| seen.insert(path.clone()))
        .filter(|path| path.exists())
        .collect()
}
