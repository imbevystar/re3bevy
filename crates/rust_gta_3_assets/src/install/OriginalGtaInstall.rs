//! re3 reference: references/re-GTA/src/core/FileMgr.cpp
//! re3 reference: references/re-GTA/src/core/FileMgr.h
//!
//! Original GTA III install root. Source code must not hardcode machine paths.

use std::{
    fs, io,
    path::{Path, PathBuf},
};

use bevy::prelude::Resource;

use super::FileMgr::{CFileMgr, resolve_existing_case_path_from, resolve_gta_path_from};

#[derive(Debug, Clone, PartialEq, Eq, Resource)]
pub struct OriginalGtaInstall {
    root: PathBuf,
}

impl OriginalGtaInstall {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn from_config_file() -> io::Result<Self> {
        #[cfg(target_os = "android")]
        {
            // The APK launcher installs its bundled data before GameActivity
            // starts. Never depend on Android's process working directory.
            let files = bevy::android::ANDROID_APP
                .get()
                .and_then(|app| app.internal_data_path())
                .ok_or_else(|| {
                    io::Error::other("Android application files directory unavailable")
                })?;
            return Self::from_root(files.join("gta3"));
        }
        #[cfg(not(target_os = "android"))]
        {
            let mut args = std::env::args_os().skip(1);
            let mut explicit_root = None;
            while let Some(arg) = args.next() {
                if arg == "--gta-dir" {
                    explicit_root = Some(PathBuf::from(args.next().ok_or_else(|| {
                        io::Error::new(
                            io::ErrorKind::InvalidInput,
                            "--gta-dir requires a GTA III directory",
                        )
                    })?));
                }
            }
            let executable = std::env::current_exe()?;
            let executable_dir = executable.parent().ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::NotFound,
                    "executable directory is unavailable",
                )
            })?;
            Self::resolve_runtime_location(
                explicit_root.as_deref(),
                &std::env::current_dir()?,
                executable_dir,
            )
        }
    }

    /// One resolver shared by game startup and the developer arena. Explicit
    /// configuration errors are returned, never hidden by a different install.
    pub fn resolve_runtime_location(
        explicit_root: Option<&Path>,
        working_dir: &Path,
        executable_dir: &Path,
    ) -> io::Result<Self> {
        if let Some(root) = explicit_root {
            return Self::from_root(if root.is_absolute() {
                root.to_path_buf()
            } else {
                working_dir.join(root)
            });
        }
        // A binary placed in an original installation works independently of
        // the launcher's working directory. An adjacent config can override it.
        let executable_config = executable_dir.join("config/original_gta3_path.txt");
        if executable_config.is_file() {
            return Self::from_config_path(executable_config);
        }
        if resolve_existing_case_path_from(executable_dir, "data/gta3.dat").is_file()
            || resolve_existing_case_path_from(executable_dir, "models/gta3.img").is_file()
        {
            return Self::from_root(executable_dir);
        }
        let working_config = working_dir.join("config/original_gta3_path.txt");
        if working_config.is_file() {
            return Self::from_config_path(working_config);
        }
        Self::from_root(working_dir).map_err(|error| {
            io::Error::new(error.kind(), format!(
                "GTA III installation unavailable: {error}. Place the executable in a GTA III installation, use --gta-dir <directory>, or configure config/original_gta3_path.txt"
            ))
        })
    }

    pub fn from_config_path(path: impl AsRef<Path>) -> io::Result<Self> {
        let path = path.as_ref();
        let text = fs::read_to_string(path)?;
        let root = text
            .lines()
            .map(str::trim)
            .find(|line| !line.is_empty() && !line.starts_with('#'))
            .ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    "original GTA III path config is empty",
                )
            })?;
        let root = Path::new(root);
        Self::from_root(if root.is_absolute() {
            root.to_path_buf()
        } else {
            path.parent().unwrap_or(Path::new(".")).join(root)
        })
    }

    pub fn from_root(root: impl Into<PathBuf>) -> io::Result<Self> {
        let install = Self::new(root);
        install.validate()?;
        Ok(Self::new(fs::canonicalize(&install.root)?))
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn resolve(&self, gta_path: impl AsRef<str>) -> PathBuf {
        resolve_gta_path_from(&self.root, gta_path.as_ref())
    }

    pub fn resolve_existing_case_path(&self, gta_path: impl AsRef<str>) -> PathBuf {
        resolve_existing_case_path_from(&self.root, gta_path.as_ref())
    }

    pub fn read(&self, gta_path: impl AsRef<str>) -> io::Result<Vec<u8>> {
        fs::read(self.resolve_existing_case_path(gta_path))
    }

    pub fn open(&self, gta_path: impl AsRef<str>) -> io::Result<fs::File> {
        fs::File::open(self.resolve_existing_case_path(gta_path))
    }

    pub fn file_mgr(&self) -> CFileMgr {
        CFileMgr::initialise(self.root.clone())
    }

    pub fn gta3_img(&self) -> PathBuf {
        self.resolve_existing_case_path("MODELS\\GTA3.IMG")
    }

    pub fn gta3_dir(&self) -> PathBuf {
        self.resolve_existing_case_path("MODELS\\GTA3.DIR")
    }

    pub fn default_dat(&self) -> PathBuf {
        self.resolve_existing_case_path("DATA\\DEFAULT.DAT")
    }

    pub fn gta3_dat(&self) -> PathBuf {
        self.resolve_existing_case_path("DATA\\GTA3.DAT")
    }

    pub fn validate(&self) -> io::Result<()> {
        if !self.root.is_dir() {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!(
                    "original GTA III root does not exist: {}",
                    self.root.display()
                ),
            ));
        }

        for required in [
            "models\\coll\\peds.col",
            "models\\gta3.img",
            "models\\gta3.dir",
            "data\\default.dat",
            "data\\gta3.dat",
        ] {
            let path = self.resolve_existing_case_path(required);
            if !path.is_file() {
                return Err(io::Error::new(
                    io::ErrorKind::NotFound,
                    format!(
                        "missing original GTA III file `{required}` under {}",
                        self.root.display()
                    ),
                ));
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn standalone_install_config_and_explicit_path_have_deterministic_precedence() {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let temporary =
            std::env::temp_dir().join(format!("rust-gta-3-install-{}-{unique}", std::process::id()));
        let install_dir = temporary.join("game");
        let launcher_dir = temporary.join("launcher");
        fs::create_dir_all(launcher_dir.join("config")).unwrap();
        for file in [
            "models/coll/peds.col",
            "models/gta3.img",
            "models/gta3.dir",
            "data/default.dat",
            "data/gta3.dat",
        ] {
            let target = install_dir.join(file);
            fs::create_dir_all(target.parent().unwrap()).unwrap();
            fs::write(target, []).unwrap();
        }
        // A copied executable must ignore an unrelated launcher's config.
        fs::write(
            launcher_dir.join("config/original_gta3_path.txt"),
            "missing",
        )
        .unwrap();
        let found = OriginalGtaInstall::resolve_runtime_location(None, &launcher_dir, &install_dir)
            .unwrap();
        assert_eq!(found.root(), fs::canonicalize(&install_dir).unwrap());
        // Explicit invalid data never falls back to the valid adjacent install.
        assert!(
            OriginalGtaInstall::resolve_runtime_location(
                Some(&temporary.join("missing")),
                &launcher_dir,
                &install_dir
            )
            .is_err()
        );
        assert!(
            OriginalGtaInstall::resolve_runtime_location(None, &launcher_dir, &launcher_dir)
                .is_err()
        );
        fs::write(
            launcher_dir.join("config/original_gta3_path.txt"),
            "# Original GTA III installation\n\n../../game\n",
        )
        .unwrap();
        let found =
            OriginalGtaInstall::resolve_runtime_location(None, &launcher_dir, &launcher_dir)
                .unwrap();
        assert_eq!(found.root(), fs::canonicalize(&install_dir).unwrap());
        assert_eq!(
            OriginalGtaInstall::resolve_runtime_location(
                Some(Path::new("game")),
                &temporary,
                &launcher_dir
            )
            .unwrap(),
            found
        );
        fs::remove_dir_all(temporary).unwrap();
    }
}
