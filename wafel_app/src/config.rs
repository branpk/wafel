use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use wafel_api::SM64Version;

pub(crate) fn libsm64_path(version: SM64Version) -> String {
    format!("libsm64/sm64_{}.dll", version.to_string().to_lowercase())
}

pub(crate) fn libsm64_locked_path(version: SM64Version) -> String {
    format!(
        "libsm6/sm64_{}.dll.locked",
        version.to_string().to_lowercase()
    )
}

pub(crate) fn is_game_version_unlocked(version: SM64Version) -> bool {
    Path::new(&libsm64_path(version)).is_file()
}

pub(crate) fn unlocked_game_versions() -> Vec<SM64Version> {
    SM64Version::all()
        .iter()
        .copied()
        .filter(|&version| is_game_version_unlocked(version))
        .collect()
}

pub(crate) fn locked_game_versions() -> Vec<SM64Version> {
    SM64Version::all()
        .iter()
        .copied()
        .filter(|&version| {
            Path::new(&libsm64_locked_path(version)).is_file() && !is_game_version_unlocked(version)
        })
        .collect()
}

pub(crate) fn default_unlocked_game_version() -> Option<SM64Version> {
    let unlocked = unlocked_game_versions();
    if unlocked.contains(&SM64Version::US) {
        Some(SM64Version::US)
    } else {
        unlocked.into_iter().next()
    }
}
