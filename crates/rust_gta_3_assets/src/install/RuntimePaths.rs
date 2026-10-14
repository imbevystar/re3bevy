//! Writable diagnostics are separate from original installation data.

/// Keep saves and settings when upgrading from either previous project name.
pub fn userfiles_root(install_root: &std::path::Path) -> std::io::Result<std::path::PathBuf> {
    let parent = install_root.parent().unwrap_or(install_root);
    let root = parent.join("rust-gta-3-userfiles");
    if !root.exists() {
        for name in ["rustgta3-userfiles", "re3bevy-userfiles"] {
            let legacy = parent.join(name);
            if legacy.is_dir() {
                std::fs::rename(&legacy, &root)?;
                break;
            }
        }
    }
    std::fs::create_dir_all(&root)?;
    Ok(root)
}

/// One writable diagnostic directory per process, independent of the checkout.
pub fn diagnostic_root() -> &'static std::path::Path {
    static ROOT: std::sync::OnceLock<std::path::PathBuf> = std::sync::OnceLock::new();
    ROOT.get_or_init(|| {
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or_default();
        #[cfg(target_os = "android")]
        let temporary = bevy::android::ANDROID_APP
            .get()
            .and_then(|app| app.internal_data_path())
            .expect("Android diagnostic path requires an active GameActivity")
            .join("diagnostics");
        #[cfg(not(target_os = "android"))]
        let temporary = std::env::temp_dir();
        temporary
            .join("rust-gta-3")
            .join(format!("diagnostics-{}-{stamp}", std::process::id()))
    })
}

#[cfg(test)]
mod tests {
    use super::userfiles_root;

    #[test]
    fn renamed_userfiles_preserve_existing_saves_and_settings() {
        for name in ["rustgta3-userfiles", "re3bevy-userfiles"] {
            let parent = super::diagnostic_root().join(name);
            let legacy = parent.join(name);
            std::fs::create_dir_all(&legacy).unwrap();
            std::fs::write(legacy.join("save.b"), b"saved game").unwrap();
            std::fs::write(legacy.join("gta3.set"), b"RE3BEVY_SETTINGS=1\nmusic=64\n").unwrap();

            let root = userfiles_root(&parent.join("GTA III")).unwrap();
            assert_eq!(root, parent.join("rust-gta-3-userfiles"));
            assert_eq!(std::fs::read(root.join("save.b")).unwrap(), b"saved game");
            assert_eq!(
                std::fs::read(root.join("gta3.set")).unwrap(),
                b"RE3BEVY_SETTINGS=1\nmusic=64\n"
            );
            assert!(!legacy.exists());
            assert_eq!(userfiles_root(&parent.join("GTA III")).unwrap(), root);
            std::fs::remove_dir_all(parent).unwrap();
        }
    }

    #[test]
    fn existing_userfiles_are_never_overwritten() {
        let parent = super::diagnostic_root().join("userfiles-existing");
        let root = parent.join("rust-gta-3-userfiles");
        let legacy = parent.join("rustgta3-userfiles");
        let older = parent.join("re3bevy-userfiles");
        std::fs::create_dir_all(&root).unwrap();
        std::fs::create_dir_all(&legacy).unwrap();
        std::fs::create_dir_all(&older).unwrap();
        std::fs::write(root.join("save.b"), b"current save").unwrap();
        std::fs::write(legacy.join("save.b"), b"legacy save").unwrap();
        std::fs::write(older.join("save.b"), b"older save").unwrap();

        assert_eq!(userfiles_root(&parent.join("GTA III")).unwrap(), root);
        assert_eq!(std::fs::read(root.join("save.b")).unwrap(), b"current save");
        assert_eq!(
            std::fs::read(legacy.join("save.b")).unwrap(),
            b"legacy save"
        );
        assert_eq!(std::fs::read(older.join("save.b")).unwrap(), b"older save");
        std::fs::remove_dir_all(parent).unwrap();
    }

    #[test]
    fn most_recent_legacy_folder_takes_precedence_without_overwriting_older_saves() {
        let parent = super::diagnostic_root().join("userfiles-both-legacy");
        let legacy = parent.join("rustgta3-userfiles");
        let older = parent.join("re3bevy-userfiles");
        std::fs::create_dir_all(&legacy).unwrap();
        std::fs::create_dir_all(&older).unwrap();
        std::fs::write(legacy.join("save.b"), b"legacy save").unwrap();
        std::fs::write(older.join("save.b"), b"older save").unwrap();

        let root = userfiles_root(&parent.join("GTA III")).unwrap();
        assert_eq!(std::fs::read(root.join("save.b")).unwrap(), b"legacy save");
        assert_eq!(std::fs::read(older.join("save.b")).unwrap(), b"older save");
        assert!(!legacy.exists());
        std::fs::remove_dir_all(parent).unwrap();
    }
}
