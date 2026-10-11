//! Writable diagnostics are separate from original installation data.

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
            .join("re3bevy")
            .join(format!("diagnostics-{}-{stamp}", std::process::id()))
    })
}
