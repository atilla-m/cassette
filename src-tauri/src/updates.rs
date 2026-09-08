//! Updater installation is authorized here, never by webview permissions.
use serde::Serialize;
use tauri::{AppHandle, Manager};

#[derive(Default)]
pub struct UpdateSession {
    available: std::sync::atomic::AtomicBool,
    #[cfg(all(target_os = "linux", feature = "signed-updater"))]
    busy: std::sync::atomic::AtomicBool,
    #[cfg(all(target_os = "linux", feature = "signed-updater"))]
    next_operation: std::sync::atomic::AtomicU64,
    #[cfg(all(target_os = "linux", feature = "signed-updater"))]
    pending: std::sync::Mutex<Option<PendingUpdate>>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeInfo {
    development: bool,
    platform: &'static str,
    package_kind: &'static str,
    can_self_install: bool,
    updater_available: bool,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInfo {
    version: String,
    body: Option<String>,
    operation_id: String,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Progress {
    downloaded_bytes: usize,
    total_bytes: Option<u64>,
    installing: bool,
}

#[cfg(all(target_os = "linux", any(test, feature = "signed-updater")))]
#[derive(Clone, Debug, PartialEq, Eq)]
struct PendingAuthorization {
    operation_id: String,
    image: detection::AppImageIdentity,
}

#[cfg(all(target_os = "linux", any(test, feature = "signed-updater")))]
fn pending_authorizes(
    pending: &PendingAuthorization,
    requested_operation: &str,
    current_image: &detection::AppImageIdentity,
) -> bool {
    pending.operation_id == requested_operation && pending.image == *current_image
}

#[cfg(all(target_os = "linux", feature = "signed-updater"))]
#[derive(Clone)]
struct PendingUpdate {
    update: tauri_plugin_updater::Update,
    authorization: PendingAuthorization,
}

#[cfg(target_os = "linux")]
mod detection {
    use std::fs;
    use std::io::Read;
    use std::os::unix::fs::MetadataExt;
    use std::path::{Path, PathBuf};

    #[derive(Clone, Debug, PartialEq, Eq)]
    pub(super) struct FileIdentity {
        pub(super) device: u64,
        pub(super) inode: u64,
        pub(super) length: u64,
        pub(super) modified_seconds: i64,
        pub(super) modified_nanoseconds: i64,
        pub(super) changed_seconds: i64,
        pub(super) changed_nanoseconds: i64,
    }

    impl FileIdentity {
        fn from_metadata(metadata: &fs::Metadata) -> Option<Self> {
            metadata.is_file().then_some(Self {
                device: metadata.dev(),
                inode: metadata.ino(),
                length: metadata.len(),
                modified_seconds: metadata.mtime(),
                modified_nanoseconds: metadata.mtime_nsec(),
                changed_seconds: metadata.ctime(),
                changed_nanoseconds: metadata.ctime_nsec(),
            })
        }
    }

    #[derive(Clone, Debug, PartialEq, Eq)]
    pub(super) struct AppImageIdentity {
        pub(super) path: PathBuf,
        pub(super) file: FileIdentity,
    }

    impl AppImageIdentity {
        pub(super) fn path(&self) -> &Path {
            &self.path
        }

        #[cfg(any(test, feature = "signed-updater"))]
        pub(super) fn still_matches_path(&self) -> bool {
            capture_path_identity(&self.path).as_ref() == Some(&self.file)
        }
    }

    fn capture_path_identity(path: &Path) -> Option<FileIdentity> {
        FileIdentity::from_metadata(&fs::metadata(path).ok()?)
    }

    fn canonical_absolute(path: &Path) -> Option<PathBuf> {
        if !path.is_absolute() {
            return None;
        }
        path.canonicalize().ok()
    }

    // Pure inputs make tests independent of process-global environment mutation.
    pub(super) fn detect(
        image: &Path,
        directory: &Path,
        executable: &Path,
    ) -> Option<AppImageIdentity> {
        let image = canonical_absolute(image)?;
        let directory = canonical_absolute(directory)?;
        let executable = canonical_absolute(executable)?;
        if !image.is_file()
            || !directory.is_dir()
            || !executable.is_file()
            || image.starts_with(&directory)
        {
            return None;
        }
        let expected_executable = directory.join("usr/bin/cassette").canonicalize().ok()?;
        let app_run = directory.join("AppRun").canonicalize().ok()?;
        let desktop = directory.join("cassette.desktop").canonicalize().ok()?;
        if expected_executable != executable
            || !executable.starts_with(&directory)
            || !app_run.starts_with(&directory)
            || !app_run.is_file()
            || !desktop.starts_with(&directory)
            || !desktop.is_file()
        {
            return None;
        }
        let desktop_text = fs::read_to_string(desktop).ok()?;
        if !desktop_text.lines().any(|line| line == "Type=Application")
            || !desktop_text.lines().any(|line| {
                line.strip_prefix("Exec=")
                    .is_some_and(|value| value.split_whitespace().next() == Some("cassette"))
            })
        {
            return None;
        }
        // Type-2 x86_64 ELF AppImage marker, independent of its filename.
        let mut image_file = fs::File::open(&image).ok()?;
        let opened_identity = FileIdentity::from_metadata(&image_file.metadata().ok()?)?;
        let mut header = [0u8; 20];
        image_file.read_exact(&mut header).ok()?;
        if &header[..4] != b"\x7fELF"
            || header[4] != 2
            || header[5] != 1
            || &header[8..11] != b"AI\x02"
            || header[18..20] != [62, 0]
        {
            return None;
        }
        // A pathname replacement between canonicalization/open/metadata must not
        // let an unrelated file become the pending self-update target.
        if capture_path_identity(&image).as_ref() != Some(&opened_identity) {
            return None;
        }
        Some(AppImageIdentity {
            path: image,
            file: opened_identity,
        })
    }

    fn mount_path(value: &str) -> Option<PathBuf> {
        let mut bytes = Vec::new();
        let mut chars = value.bytes();
        while let Some(c) = chars.next() {
            if c == b'\\' {
                let digits = [chars.next()?, chars.next()?, chars.next()?];
                if digits.iter().any(|c| !(b'0'..=b'7').contains(c)) {
                    return None;
                }
                let code = (digits[0] - b'0') as u16 * 64
                    + (digits[1] - b'0') as u16 * 8
                    + (digits[2] - b'0') as u16;
                bytes.push(u8::try_from(code).ok()?);
            } else {
                bytes.push(c);
            }
        }
        use std::os::unix::ffi::OsStringExt;
        Some(PathBuf::from(std::ffi::OsString::from_vec(bytes)))
    }

    fn mount_matches(mountinfo: &str, directory: &Path, image: &Path) -> bool {
        mountinfo.lines().any(|line| {
            let Some((left, right)) = line.split_once(" - ") else {
                return false;
            };
            let fields: Vec<_> = left.split_whitespace().collect();
            let fs_fields: Vec<_> = right.split_whitespace().collect();
            if fields.len() < 6
                || fs_fields.len() < 3
                || !fs_fields[0].starts_with("fuse")
                || !fields[5].split(',').any(|option| option == "ro")
            {
                return false;
            }
            let Some(mount) = mount_path(fields[4]).and_then(|p| canonical_absolute(&p)) else {
                return false;
            };
            let Some(source) = mount_path(fs_fields[1]).and_then(|p| canonical_absolute(&p)) else {
                return false;
            };
            mount == directory && source == image
        })
    }

    pub(super) fn current() -> Option<AppImageIdentity> {
        // Tauri's embedded package marker is independent of inherited variables.
        // Its updater dispatches DEB/RPM to package managers, so require AppImage
        // explicitly before any native download/install authorization.
        if !matches!(
            tauri::utils::platform::bundle_type(),
            Some(tauri::utils::config::BundleType::AppImage)
        ) {
            return None;
        }
        let directory = PathBuf::from(std::env::var_os("APPDIR")?);
        let image = detect(
            &PathBuf::from(std::env::var_os("APPIMAGE")?),
            &directory,
            &std::env::current_exe().ok()?,
        )?;
        // The kernel mount table binds the mounted AppDir to its backing image.
        // Extract-and-run and runtimes without identifiable FUSE evidence fall
        // back to download-only; environment variables alone never authorize.
        mount_matches(
            &fs::read_to_string("/proc/self/mountinfo").ok()?,
            &directory.canonicalize().ok()?,
            image.path(),
        )
        .then_some(image)
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use std::os::unix::fs::symlink;
        struct Fixture {
            root: PathBuf,
            image: PathBuf,
            dir: PathBuf,
            exe: PathBuf,
        }
        impl Fixture {
            fn new() -> Self {
                static NEXT: std::sync::atomic::AtomicUsize =
                    std::sync::atomic::AtomicUsize::new(0);
                let root = std::env::temp_dir().join(format!(
                    "cassette-updater-{}-{}",
                    std::process::id(),
                    NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
                ));
                let dir = root.join("AppDir");
                let exe = dir.join("usr/bin/cassette");
                fs::create_dir_all(exe.parent().unwrap()).unwrap();
                fs::write(&exe, b"test executable").unwrap();
                fs::write(dir.join("AppRun"), b"test launcher").unwrap();
                fs::write(
                    dir.join("cassette.desktop"),
                    b"[Desktop Entry]\nType=Application\nExec=cassette %U\n",
                )
                .unwrap();
                let image = root.join("application-without-extension");
                let mut header = [0u8; 20];
                header[..4].copy_from_slice(b"\x7fELF");
                header[4] = 2;
                header[5] = 1;
                header[8..11].copy_from_slice(b"AI\x02");
                header[18] = 62;
                fs::write(&image, header).unwrap();
                Self {
                    root,
                    image,
                    dir,
                    exe,
                }
            }
            fn valid(&self) -> bool {
                detect(&self.image, &self.dir, &self.exe).is_some()
            }
        }
        impl Drop for Fixture {
            fn drop(&mut self) {
                let _ = fs::remove_dir_all(&self.root);
            }
        }
        #[test]
        fn valid_structure_without_filename_extension() {
            assert!(Fixture::new().valid());
        }
        #[test]
        fn unchanged_image_keeps_filesystem_identity() {
            let f = Fixture::new();
            let identity = detect(&f.image, &f.dir, &f.exe).unwrap();
            assert!(identity.still_matches_path());
            assert_eq!(identity.path(), f.image.canonicalize().unwrap());
        }
        #[test]
        fn replacing_image_at_same_path_changes_filesystem_identity() {
            let f = Fixture::new();
            let identity = detect(&f.image, &f.dir, &f.exe).unwrap();
            let original = f.root.join("original-image");
            fs::rename(&f.image, original).unwrap();
            let mut replacement = [0u8; 20];
            replacement[..4].copy_from_slice(b"\x7fELF");
            replacement[4] = 2;
            replacement[5] = 1;
            replacement[8..11].copy_from_slice(b"AI\x02");
            replacement[18] = 62;
            fs::write(&f.image, replacement).unwrap();
            assert!(!identity.still_matches_path());
        }
        #[test]
        fn renaming_different_file_over_image_changes_filesystem_identity() {
            let f = Fixture::new();
            let identity = detect(&f.image, &f.dir, &f.exe).unwrap();
            let replacement = f.root.join("replacement-image");
            fs::write(&replacement, fs::read(&f.image).unwrap()).unwrap();
            fs::rename(replacement, &f.image).unwrap();
            assert!(!identity.still_matches_path());
        }
        #[test]
        fn mount_source_must_match_image_and_directory() {
            let f = Fixture::new();
            let other = Fixture::new();
            let mount = format!(
                "123 45 0:99 / {} ro,nosuid - fuse.AppImage {} ro",
                f.dir.display(),
                f.image.display()
            );
            assert!(mount_matches(&mount, &f.dir, &f.image));
            assert!(!mount_matches(&mount, &f.dir, &other.image));
            assert!(!mount_matches(&mount, &other.dir, &f.image));
            assert!(!mount_matches("", &f.dir, &f.image));
        }
        #[test]
        fn arbitrary_file_directory_rejected() {
            let f = Fixture::new();
            fs::write(&f.image, b"not an AppImage").unwrap();
            assert!(!f.valid());
        }
        #[test]
        fn stale_native_launch_rejected() {
            let f = Fixture::new();
            let native = f.root.join("native-cassette");
            fs::write(&native, b"native").unwrap();
            assert!(detect(&f.image, &f.dir, &native).is_none());
        }
        #[test]
        fn relative_and_missing_paths_rejected() {
            let f = Fixture::new();
            assert!(detect(Path::new("image"), &f.dir, &f.exe).is_none());
            assert!(detect(&f.image, Path::new("AppDir"), &f.exe).is_none());
            assert!(detect(&f.root.join("missing"), &f.dir, &f.exe).is_none());
            assert!(detect(&f.image, &f.root.join("missing"), &f.exe).is_none());
        }
        #[test]
        fn deleted_image_rejected() {
            let f = Fixture::new();
            fs::remove_file(&f.image).unwrap();
            assert!(!f.valid());
        }
        #[test]
        fn escaped_executable_symlink_rejected() {
            let f = Fixture::new();
            let outside = f.root.join("outside");
            fs::write(&outside, b"native").unwrap();
            fs::remove_file(&f.exe).unwrap();
            symlink(&outside, &f.exe).unwrap();
            assert!(!f.valid());
        }
        #[test]
        fn escaped_launcher_and_mismatched_directory_rejected() {
            let f = Fixture::new();
            fs::remove_file(f.dir.join("AppRun")).unwrap();
            symlink(&f.image, f.dir.join("AppRun")).unwrap();
            assert!(!f.valid());
            let other = Fixture::new();
            assert!(detect(&f.image, &other.dir, &f.exe).is_none());
        }
    }
}

#[cfg(all(test, target_os = "linux"))]
mod pending_authorization_tests {
    use super::*;
    use std::fs;
    use std::os::unix::fs::MetadataExt;

    fn image_identity(path: &std::path::Path) -> detection::AppImageIdentity {
        let path = path.canonicalize().unwrap();
        let metadata = fs::metadata(&path).unwrap();
        detection::AppImageIdentity {
            path,
            file: detection::FileIdentity {
                device: metadata.dev(),
                inode: metadata.ino(),
                length: metadata.len(),
                modified_seconds: metadata.mtime(),
                modified_nanoseconds: metadata.mtime_nsec(),
                changed_seconds: metadata.ctime(),
                changed_nanoseconds: metadata.ctime_nsec(),
            },
        }
    }

    #[test]
    fn stale_operation_identifier_is_rejected() {
        let file = tempfile_path("stale-operation");
        fs::write(&file, b"image").unwrap();
        let image = image_identity(&file);
        let pending = PendingAuthorization {
            operation_id: "new-operation".into(),
            image: image.clone(),
        };
        assert!(!pending_authorizes(&pending, "old-operation", &image));
        assert!(pending_authorizes(&pending, "new-operation", &image));
        fs::remove_file(file).unwrap();
    }

    #[test]
    fn same_version_pending_replacement_rejects_old_operation() {
        // Version is intentionally absent from authorization. A second check for
        // the same version gets a new opaque operation id, invalidating the old UI.
        let file = tempfile_path("same-version-operation");
        fs::write(&file, b"image").unwrap();
        let image = image_identity(&file);
        let replacement = PendingAuthorization {
            operation_id: "operation-2".into(),
            image: image.clone(),
        };
        assert!(!pending_authorizes(&replacement, "operation-1", &image));
        assert!(pending_authorizes(&replacement, "operation-2", &image));
        fs::remove_file(file).unwrap();
    }

    fn tempfile_path(label: &str) -> std::path::PathBuf {
        static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        std::env::temp_dir().join(format!(
            "cassette-{label}-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ))
    }
}

pub fn initialize(app: &AppHandle) {
    app.manage(UpdateSession::default());
    #[cfg(all(target_os = "linux", feature = "signed-updater"))]
    {
        use tauri_plugin_updater::UpdaterExt;
        // Do not interpolate configuration/error values: they may contain secrets.
        let ready = valid_configuration(app.config().plugins.0.get("updater"))
            && app
                .plugin(tauri_plugin_updater::Builder::new().build())
                .is_ok()
            && app.updater().is_ok();
        if !ready {
            eprintln!("Cassette updater unavailable: plugin/configuration initialization failed; application startup continues.");
        }
        app.state::<UpdateSession>()
            .available
            .store(ready, std::sync::atomic::Ordering::Release);
    }
}

#[cfg(all(target_os = "linux", feature = "signed-updater"))]
fn valid_configuration(value: Option<&serde_json::Value>) -> bool {
    use base64::Engine;
    let Some(value) = value else {
        return false;
    };
    let Some(key) = value.get("pubkey").and_then(|v| v.as_str()) else {
        return false;
    };
    let Ok(decoded) = base64::engine::general_purpose::STANDARD.decode(key.trim()) else {
        return false;
    };
    let Ok(key_text) = std::str::from_utf8(&decoded) else {
        return false;
    };
    if minisign_verify::PublicKey::decode(key_text).is_err() {
        return false;
    }
    let Some(endpoints) = value.get("endpoints").and_then(|v| v.as_array()) else {
        return false;
    };
    if endpoints.len() != 1 {
        return false;
    }
    let Some(endpoint) = endpoints[0]
        .as_str()
        .and_then(|s| reqwest::Url::parse(s).ok())
    else {
        return false;
    };
    endpoint.scheme() == "https"
        && endpoint.host_str() == Some("atilla-m.github.io")
        && endpoint.path() == "/cassette/updates/beta/latest.json"
        && endpoint.username().is_empty()
        && endpoint.password().is_none()
        && endpoint.port().is_none()
        && endpoint.query().is_none()
        && endpoint.fragment().is_none()
        && !value.as_object().is_some_and(|o| {
            o.iter()
                .any(|(key, value)| key.starts_with("dangerous") && value.as_bool() == Some(true))
        })
}

#[tauri::command]
pub fn get_update_runtime_info(app: AppHandle) -> RuntimeInfo {
    let available = app
        .state::<UpdateSession>()
        .available
        .load(std::sync::atomic::Ordering::Acquire);
    #[cfg(target_os = "linux")]
    let appimage = detection::current().is_some();
    #[cfg(not(target_os = "linux"))]
    let appimage = false;
    RuntimeInfo {
        development: cfg!(debug_assertions),
        platform: std::env::consts::OS,
        package_kind: if !cfg!(target_os = "linux") {
            "unsupported"
        } else if appimage {
            "appimage"
        } else {
            "native-or-unknown"
        },
        can_self_install: cfg!(target_os = "linux")
            && !cfg!(debug_assertions)
            && available
            && appimage,
        updater_available: available,
    }
}

#[cfg(all(target_os = "linux", feature = "signed-updater"))]
mod enabled {
    use super::*;
    use std::sync::atomic::Ordering;
    use tauri_plugin_updater::UpdaterExt;
    pub(super) struct Busy<'a>(&'a std::sync::atomic::AtomicBool);
    impl Drop for Busy<'_> {
        fn drop(&mut self) {
            self.0.store(false, Ordering::Release);
        }
    }
    pub(super) fn enter(session: &UpdateSession) -> Result<Busy<'_>, String> {
        if cfg!(debug_assertions) || !session.available.load(Ordering::Acquire) {
            return Err("Updates are unavailable in this build or session; updater configuration could not be initialized.".into());
        }
        session
            .busy
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .map_err(|_| "An update operation is already in progress.".to_string())?;
        Ok(Busy(&session.busy))
    }
    pub(super) async fn check(app: AppHandle) -> Result<Option<UpdateInfo>, String> {
        let session = app.state::<UpdateSession>();
        let _busy = enter(&session)?;
        let image = detection::current();
        let executable = image
            .as_ref()
            .map(|identity| identity.path().to_path_buf())
            .or_else(|| std::env::current_exe().ok())
            .ok_or("Cannot determine executable path.")?;
        let update = app
            .updater_builder()
            .executable_path(executable)
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|_| "Updater configuration is unavailable.")?
            .check()
            .await
            .map_err(|_| {
                "Could not reach or read the beta update feed. Check your connection and try again."
            })?;
        let (info, pending) = match (update, image) {
            (Some(update), Some(image)) => {
                let sequence = session
                    .next_operation
                    .fetch_update(Ordering::AcqRel, Ordering::Acquire, |value| {
                        value.checked_add(1)
                    })
                    .map_err(|_| "Updater operation identifier space is exhausted.")?
                    + 1;
                let operation_id = format!("update-{sequence:016x}");
                let info = UpdateInfo {
                    version: update.version.clone(),
                    body: update.body.clone(),
                    operation_id: operation_id.clone(),
                };
                let pending = PendingUpdate {
                    update,
                    authorization: PendingAuthorization {
                        operation_id,
                        image,
                    },
                };
                (Some(info), Some(pending))
            }
            (Some(update), None) => {
                // Native package/unknown installs may still learn that an update
                // exists, but they can never hold an installable pending update.
                (
                    Some(UpdateInfo {
                        version: update.version,
                        body: update.body,
                        operation_id: String::new(),
                    }),
                    None,
                )
            }
            (None, _) => (None, None),
        };
        *session
            .pending
            .lock()
            .map_err(|_| "Updater state unavailable.")? = pending;
        Ok(info)
    }
    pub(super) async fn install(
        app: AppHandle,
        operation_id: String,
        progress: tauri::ipc::Channel<Progress>,
    ) -> Result<(), String> {
        let session = app.state::<UpdateSession>();
        let _busy = enter(&session)?;
        let image = detection::current().ok_or(
            "This installation is not a verified AppImage. Use the release download page.",
        )?;
        let pending = session
            .pending
            .lock()
            .map_err(|_| "Updater state unavailable.")?
            .clone()
            .filter(|pending| {
                pending_authorizes(&pending.authorization, &operation_id, &image)
                    && pending.authorization.image.still_matches_path()
            })
            .ok_or("The selected update or AppImage changed. Check for updates again.")?;
        let PendingUpdate {
            mut update,
            authorization,
        } = pending;
        update.timeout = Some(std::time::Duration::from_secs(300));
        let mut downloaded = 0;
        let bytes = update
            .download(
                |count, total| {
                    downloaded += count;
                    let _ = progress.send(Progress {
                        downloaded_bytes: downloaded,
                        total_bytes: total,
                        installing: false,
                    });
                },
                || {},
            )
            .await
            .map_err(|_| "Update download or signature verification failed. You can retry.")?;
        // Reauthorize after network I/O, immediately before the native installer.
        if detection::current().as_ref() != Some(&authorization.image)
            || !authorization.image.still_matches_path()
        {
            return Err(
                "AppImage runtime changed during download; installation was refused.".into(),
            );
        }
        let _ = progress.send(Progress {
            downloaded_bytes: bytes.len(),
            total_bytes: Some(bytes.len() as u64),
            installing: true,
        });
        update
            .install(bytes)
            .map_err(|_| "AppImage installation failed. Check the application file and retry.")?;
        *session
            .pending
            .lock()
            .map_err(|_| "Updater state unavailable.")? = None;
        Ok(())
    }
}

#[tauri::command]
pub async fn check_cassette_update(app: AppHandle) -> Result<Option<UpdateInfo>, String> {
    #[cfg(all(target_os = "linux", feature = "signed-updater"))]
    {
        enabled::check(app).await
    }
    #[cfg(not(all(target_os = "linux", feature = "signed-updater")))]
    {
        let _ = app;
        Err("Updates are unavailable in this build; signed Linux updater configuration is required.".into())
    }
}

#[tauri::command]
pub async fn install_cassette_update(
    app: AppHandle,
    operation_id: String,
    progress: tauri::ipc::Channel<Progress>,
) -> Result<(), String> {
    #[cfg(all(target_os = "linux", feature = "signed-updater"))]
    {
        enabled::install(app, operation_id, progress).await
    }
    #[cfg(not(all(target_os = "linux", feature = "signed-updater")))]
    {
        let _ = (app, operation_id, progress);
        Err("Update installation is unavailable on this platform/build.".into())
    }
}
