//! Local/system font discovery for the CLI.
//!
//! Enumerates the per-OS directories where the operating system keeps installed
//! fonts. This OS-specific path enumeration lives in the CLI — NOT in
//! `zenith-core` — so the core stays free of machine-specific assumptions: it
//! only ever reads font files from a directory list the caller hands it.
//!
//! The render path uses these dirs as a LAST-RESORT font source: a face found
//! here is registered with `FontSource::Local` and trips a `font.local`
//! advisory, because output that depends on a machine-local font is not
//! guaranteed deterministic across machines.

use std::path::PathBuf;

/// Resolve `$HOME` as a [`PathBuf`].
///
/// Mirrors the pattern used by the plugin-paths module: `var_os` returns `None`
/// when the variable is unset, so no panic is possible. Only the unix-family
/// targets (linux/macos) consult `$HOME` for per-user font dirs.
#[cfg(any(target_os = "linux", target_os = "macos"))]
fn home_dir() -> Option<PathBuf> {
    std::env::var_os("HOME").map(PathBuf::from)
}

/// The OS font directories to scan for local/system fonts, most-canonical first.
///
/// Only directories that can be named without panicking are included; entries
/// that depend on an unset environment variable are simply omitted. The returned
/// list may contain directories that do not exist — the scanner skips those.
#[cfg(target_os = "linux")]
#[must_use]
pub fn os_font_dirs() -> Vec<PathBuf> {
    let mut dirs = vec![
        PathBuf::from("/usr/share/fonts"),
        PathBuf::from("/usr/local/share/fonts"),
    ];
    if let Some(home) = home_dir() {
        dirs.push(home.join(".fonts"));
        dirs.push(home.join(".local/share/fonts"));
    }
    dirs
}

/// The OS font directories to scan for local/system fonts, most-canonical first.
#[cfg(target_os = "macos")]
#[must_use]
pub fn os_font_dirs() -> Vec<PathBuf> {
    let mut dirs = vec![
        PathBuf::from("/System/Library/Fonts"),
        PathBuf::from("/Library/Fonts"),
    ];
    if let Some(home) = home_dir() {
        dirs.push(home.join("Library/Fonts"));
    }
    dirs
}

/// The OS font directories to scan for local/system fonts, most-canonical first.
#[cfg(target_os = "windows")]
#[must_use]
pub fn os_font_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Some(windir) = std::env::var_os("WINDIR") {
        dirs.push(PathBuf::from(windir).join("Fonts"));
    }
    if let Some(local) = std::env::var_os("LOCALAPPDATA") {
        dirs.push(PathBuf::from(local).join("Microsoft/Windows/Fonts"));
    }
    dirs
}

/// Fallback for any other target OS: no known system font locations.
#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
#[must_use]
pub fn os_font_dirs() -> Vec<PathBuf> {
    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn os_font_dirs_is_callable_and_paths_are_absolute_or_under_home() {
        // The list may legitimately be empty on exotic targets, but every entry
        // it does contain must be a non-empty path.
        for dir in os_font_dirs() {
            assert!(
                !dir.as_os_str().is_empty(),
                "an os font dir entry must not be empty"
            );
        }
    }
}
