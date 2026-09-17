//! Per-user launcher for a verified, FUSE-mounted AppImage only.
//! This deliberately uses a different desktop-file ID from DEB/RPM's Cassette.desktop.

use super::detection::{self, AppImageIdentity};
use super::MenuStatus;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

const DESKTOP_ID: &str = "io.github.atilla.cassette.AppImage.desktop";
const ICON_NAME: &str = "io.github.atilla.cassette.AppImage";
const ICON_BYTES: &[u8] = include_bytes!("../../icons/128x128.png");
const MANAGED_MARKER: &str = "X-Cassette-AppImage-Managed=true";
static NEXT_TEMP_FILE: AtomicU64 = AtomicU64::new(0);

struct MenuPaths {
    desktop: PathBuf,
    icon: PathBuf,
}

impl MenuPaths {
    fn for_data_home(data_home: &Path) -> Self {
        Self {
            desktop: data_home.join("applications").join(DESKTOP_ID),
            icon: data_home
                .join("icons/hicolor/128x128/apps")
                .join(format!("{ICON_NAME}.png")),
        }
    }

    fn current() -> Result<Self, String> {
        let data_home = std::env::var_os("XDG_DATA_HOME")
            .filter(|value| !value.is_empty())
            .map(PathBuf::from)
            .filter(|path| path.is_absolute())
            .or_else(|| {
                std::env::var_os("HOME")
                    .map(PathBuf::from)
                    .filter(|path| path.is_absolute())
                    .map(|home| home.join(".local/share"))
            })
            .ok_or("Cannot determine the current user's XDG data directory.")?;
        Ok(Self::for_data_home(&data_home))
    }
}

fn verified_image() -> Result<AppImageIdentity, String> {
    if cfg!(debug_assertions) {
        return Err("Applications-menu setup is available only from a packaged AppImage.".into());
    }
    detection::current().ok_or_else(|| {
        "This launch is not a verified, FUSE-mounted AppImage; no applications-menu files were changed."
            .into()
    })
}

fn image_path(image: &AppImageIdentity) -> Result<&str, String> {
    image
        .path()
        .to_str()
        .ok_or_else(|| "This AppImage path cannot be represented in a desktop entry.".into())
}

// Encode the Exec value for both desktop-file-validate and GIO's desktop
// launcher. Percent signs are field codes and must be doubled; the other
// reserved characters must stay inside the quoted executable/argument.
fn exec_argument(path: &str) -> Result<String, String> {
    if !path.starts_with('/') || path.chars().any(char::is_control) {
        return Err("The AppImage path is not a safe absolute desktop-entry path.".into());
    }
    let mut result = String::from("\"");
    for character in path.chars() {
        match character {
            '%' => result.push_str("%%"),
            '\\' => result.push_str("\\\\\\\\"),
            '"' => result.push_str("\\\\\""),
            '$' | '`' => {
                result.push_str("\\\\");
                result.push(character);
            }
            _ => result.push(character),
        }
    }
    result.push('"');
    Ok(result)
}

fn desktop_entry(image_path: &str) -> Result<String, String> {
    // The desktop-entry specification forbids '=' in the executable position;
    // GIO also rejects '%' field codes there. util-linux setsid takes the path
    // as a literal argument and execs it, without invoking a shell. Ordinary
    // paths launch the AppImage directly.
    let prefix = if image_path.contains(['=', '%']) {
        "/usr/bin/setsid -- "
    } else {
        ""
    };
    Ok(format!(
        "[Desktop Entry]\nType=Application\nName=Cassette (AppImage)\nComment=Local-first music player\nExec={prefix}{}\nIcon={ICON_NAME}\nTerminal=false\nCategories=AudioVideo;Audio;Player;\n{MANAGED_MARKER}\n",
        exec_argument(image_path)?
    ))
}

fn file_bytes_if_regular(path: &Path) -> Result<Option<Vec<u8>>, String> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(format!("Cannot inspect {}: {error}", path.display())),
    };
    if !metadata.file_type().is_file() {
        return Err(format!(
            "Refusing to change {} because it is not a regular file.",
            path.display()
        ));
    }
    fs::read(path)
        .map(Some)
        .map_err(|error| format!("Cannot read {}: {error}", path.display()))
}

fn managed_desktop(path: &Path) -> Result<Option<String>, String> {
    let Some(bytes) = file_bytes_if_regular(path)? else {
        return Ok(None);
    };
    let text = String::from_utf8(bytes).map_err(|_| {
        format!(
            "The existing launcher at {} is not UTF-8; it was left unchanged.",
            path.display()
        )
    })?;
    let lines: Vec<_> = text.lines().collect();
    let managed = lines.len() == 9
        && lines[0] == "[Desktop Entry]"
        && lines[1] == "Type=Application"
        && lines[2] == "Name=Cassette (AppImage)"
        && lines[3] == "Comment=Local-first music player"
        && (lines[4].starts_with("Exec=\"") || lines[4].starts_with("Exec=/usr/bin/setsid -- \""))
        && lines[5] == format!("Icon={ICON_NAME}")
        && lines[6] == "Terminal=false"
        && lines[7] == "Categories=AudioVideo;Audio;Player;"
        && lines[8] == MANAGED_MARKER;
    if !managed {
        return Err(format!(
            "An unrelated launcher already exists at {}; it was left unchanged.",
            path.display()
        ));
    }
    Ok(Some(text))
}

fn icon_state(path: &Path) -> Result<bool, String> {
    match file_bytes_if_regular(path)? {
        None => Ok(false),
        Some(bytes) if bytes == ICON_BYTES => Ok(true),
        Some(_) => Err(format!(
            "A different icon already exists at {}; it was left unchanged.",
            path.display()
        )),
    }
}

fn status_at(image: &AppImageIdentity, paths: &MenuPaths) -> Result<MenuStatus, String> {
    let image_path = image_path(image)?;
    let expected = desktop_entry(image_path)?;
    let existing = managed_desktop(&paths.desktop)?;
    let icon_matches = icon_state(&paths.icon)?;
    let installed = existing.is_some();
    Ok(MenuStatus {
        installed,
        needs_refresh: installed
            && (existing.as_ref().is_some_and(|text| text != &expected) || !icon_matches),
        image_path: image_path.to_owned(),
    })
}

fn ensure_directory(path: &Path) -> Result<(), String> {
    if let Ok(metadata) = fs::symlink_metadata(path) {
        if !metadata.is_dir() || metadata.file_type().is_symlink() {
            return Err(format!(
                "Refusing to use non-directory or symlinked destination {}.",
                path.display()
            ));
        }
        return Ok(());
    }
    fs::create_dir(path).map_err(|error| format!("Cannot create {}: {error}", path.display()))
}

fn ensure_parent_directories(data_home: &Path, parent: &Path) -> Result<(), String> {
    fs::create_dir_all(data_home).map_err(|error| {
        format!(
            "Cannot create XDG data directory {}: {error}",
            data_home.display()
        )
    })?;
    let relative = parent
        .strip_prefix(data_home)
        .map_err(|_| "Applications-menu destination escaped the XDG data directory.")?;
    let mut directory = data_home.to_path_buf();
    for component in relative.components() {
        directory.push(component);
        ensure_directory(&directory)?;
    }
    Ok(())
}

fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or("Applications-menu destination has no parent directory.")?;
    let temporary = parent.join(format!(
        ".cassette-appimage-{}-{}",
        std::process::id(),
        NEXT_TEMP_FILE.fetch_add(1, Ordering::Relaxed)
    ));
    let result = (|| {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)
            .map_err(|error| format!("Cannot stage {}: {error}", path.display()))?;
        file.set_permissions(fs::Permissions::from_mode(0o644))
            .map_err(|error| format!("Cannot set permissions for {}: {error}", path.display()))?;
        file.write_all(bytes)
            .and_then(|_| file.sync_all())
            .map_err(|error| format!("Cannot write {}: {error}", path.display()))?;
        fs::rename(&temporary, path)
            .map_err(|error| format!("Cannot install {}: {error}", path.display()))
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

fn install_at<F>(
    image: &AppImageIdentity,
    paths: &MenuPaths,
    still_verified: F,
) -> Result<MenuStatus, String>
where
    F: Fn() -> bool,
{
    let image_path = image_path(image)?;
    let entry = desktop_entry(image_path)?;
    if image_path.contains(['=', '%']) {
        let launcher = Path::new("/usr/bin/setsid");
        let available = fs::metadata(launcher).ok().is_some_and(|metadata| {
            metadata.is_file() && metadata.permissions().mode() & 0o111 != 0
        });
        if !available {
            return Err("This AppImage path needs /usr/bin/setsid for desktop-entry escaping, but it is unavailable; no launcher was installed.".into());
        }
    }
    let existing = managed_desktop(&paths.desktop)?;
    let icon_matches = icon_state(&paths.icon)?;
    if existing.is_none() && icon_matches {
        return Err(format!(
            "An icon already exists at {} without Cassette's managed launcher; it was left unchanged.",
            paths.icon.display()
        ));
    }
    if !still_verified() {
        return Err(
            "The running AppImage changed; no launcher was installed. Restart and retry.".into(),
        );
    }
    let data_home = paths
        .desktop
        .parent()
        .and_then(Path::parent)
        .ok_or("Cannot determine the XDG data directory.")?;
    ensure_parent_directories(data_home, paths.icon.parent().unwrap())?;
    ensure_parent_directories(data_home, paths.desktop.parent().unwrap())?;

    if existing.as_deref() != Some(&entry) {
        if !still_verified() {
            return Err(
                "The running AppImage changed; no launcher was installed. Restart and retry."
                    .into(),
            );
        }
        atomic_write(&paths.desktop, entry.as_bytes())?;
    }
    if !icon_matches {
        if let Err(error) = atomic_write(&paths.icon, ICON_BYTES) {
            if existing.is_none()
                && managed_desktop(&paths.desktop).ok().flatten().as_deref() == Some(&entry)
            {
                let _ = fs::remove_file(&paths.desktop);
            }
            return Err(error);
        }
    }
    status_at(image, paths)
}

fn remove_at(image: &AppImageIdentity, paths: &MenuPaths) -> Result<MenuStatus, String> {
    let existing = managed_desktop(&paths.desktop)?;
    if existing.is_none() {
        return Err("Cassette's managed AppImage launcher is not installed.".into());
    }
    let icon_matches = icon_state(&paths.icon)?;
    if icon_matches {
        fs::remove_file(&paths.icon)
            .map_err(|error| format!("Cannot remove {}: {error}", paths.icon.display()))?;
    }
    // Keep the managed desktop entry until last so a failed partial removal
    // can be retried instead of leaving an unowned icon behind.
    fs::remove_file(&paths.desktop)
        .map_err(|error| format!("Cannot remove {}: {error}", paths.desktop.display()))?;
    status_at(image, paths)
}

pub(super) fn status() -> Result<MenuStatus, String> {
    let image = verified_image()?;
    status_at(&image, &MenuPaths::current()?)
}

pub(super) fn install() -> Result<MenuStatus, String> {
    let image = verified_image()?;
    let paths = MenuPaths::current()?;
    install_at(&image, &paths, || {
        image.still_matches_path() && detection::current().as_ref() == Some(&image)
    })
}

pub(super) fn remove() -> Result<MenuStatus, String> {
    let image = verified_image()?;
    remove_at(&image, &MenuPaths::current()?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::symlink;
    use std::process::Command;

    struct Fixture {
        root: PathBuf,
        app_dir: PathBuf,
        executable: PathBuf,
        image: PathBuf,
        paths: MenuPaths,
    }

    impl Fixture {
        fn new(image_name: &str) -> Self {
            static NEXT: AtomicU64 = AtomicU64::new(0);
            let root = std::env::temp_dir().join(format!(
                "cassette-menu-test-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir(&root).unwrap();
            let app_dir = root.join("mounted-appdir");
            let executable = app_dir.join("usr/bin/cassette");
            fs::create_dir_all(executable.parent().unwrap()).unwrap();
            fs::write(&executable, b"fixture executable").unwrap();
            fs::write(app_dir.join("AppRun"), b"fixture launcher").unwrap();
            let packaged_desktop = app_dir.join("usr/share/applications/Cassette.desktop");
            fs::create_dir_all(packaged_desktop.parent().unwrap()).unwrap();
            fs::write(
                &packaged_desktop,
                b"[Desktop Entry]\nType=Application\nExec=cassette %U\n",
            )
            .unwrap();
            symlink(
                "usr/share/applications/Cassette.desktop",
                app_dir.join("Cassette.desktop"),
            )
            .unwrap();
            let image = root.join(image_name);
            Self::write_image(&image);
            let paths = MenuPaths::for_data_home(&root.join("isolated-xdg-data"));
            Self {
                root,
                app_dir,
                executable,
                image,
                paths,
            }
        }

        fn write_image(path: &Path) {
            let mut header = [0u8; 20];
            header[..4].copy_from_slice(b"\x7fELF");
            header[4] = 2;
            header[5] = 1;
            header[8..11].copy_from_slice(b"AI\x02");
            header[18] = 62;
            fs::write(path, header).unwrap();
        }

        fn identity(&self) -> AppImageIdentity {
            detection::detect(&self.image, &self.app_dir, &self.executable).unwrap()
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    #[test]
    fn exec_escapes_reserved_characters_and_rejects_controls() {
        assert_eq!(
            exec_argument("/images/Cassette space%.AppImage").unwrap(),
            "\"/images/Cassette space%%.AppImage\""
        );
        assert_eq!(
            exec_argument("/images/Cassette $x").unwrap(),
            "\"/images/Cassette \\\\$x\""
        );
        assert!(exec_argument("relative.AppImage").is_err());
        assert!(exec_argument("/images/new\nline.AppImage").is_err());
    }

    #[test]
    fn desktop_validator_and_gio_launch_accept_special_executable_path() {
        if !Path::new("/usr/bin/setsid").is_file()
            || Command::new("desktop-file-validate")
                .arg("--version")
                .output()
                .is_err()
            || Command::new("gio").arg("help").output().is_err()
        {
            // The pure escaping test still runs on minimal CI images.
            return;
        }
        let fixture = Fixture::new("Cassette.AppImage");
        fs::create_dir_all(fixture.paths.desktop.parent().unwrap()).unwrap();
        for (index, name) in [
            "Cassette space.AppImage",
            "Cassette $.AppImage",
            "Cassette `.AppImage",
            r"Cassette \#.AppImage",
            "Cassette \".AppImage",
            "Cassette %.AppImage",
            "Cassette =.AppImage",
            "Cassette $`\\\"%#;&' =.AppImage",
            "Cassette 日本語 €.AppImage",
        ]
        .into_iter()
        .enumerate()
        {
            let special = fixture.root.join(name);
            let marker = fixture.root.join(format!("launched-{index}"));
            fs::write(
                &special,
                format!("#!/bin/sh\nprintf launched > '{}'\n", marker.display()),
            )
            .unwrap();
            fs::set_permissions(&special, fs::Permissions::from_mode(0o755)).unwrap();
            fs::write(
                &fixture.paths.desktop,
                desktop_entry(special.to_str().unwrap()).unwrap(),
            )
            .unwrap();

            let validated = Command::new("desktop-file-validate")
                .arg(&fixture.paths.desktop)
                .output()
                .unwrap();
            assert!(
                validated.status.success(),
                "{name}: desktop-file-validate: stdout={} stderr={}",
                String::from_utf8_lossy(&validated.stdout),
                String::from_utf8_lossy(&validated.stderr)
            );
            let launched = Command::new("gio")
                .arg("launch")
                .arg(&fixture.paths.desktop)
                .output()
                .unwrap();
            assert!(
                launched.status.success(),
                "{name}: gio launch: {}",
                String::from_utf8_lossy(&launched.stderr)
            );
            for _ in 0..20 {
                if marker.exists() {
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(50));
            }
            assert!(marker.exists(), "{name}: gio did not launch the exact path");
        }
    }

    #[test]
    fn setup_is_repeatable_and_removal_preserves_package_entry() {
        let fixture = Fixture::new("Cassette 100%.AppImage");
        let image = fixture.identity();
        let package_entry = fixture
            .paths
            .desktop
            .parent()
            .unwrap()
            .join("Cassette.desktop");
        fs::create_dir_all(package_entry.parent().unwrap()).unwrap();
        fs::write(&package_entry, b"package-managed entry").unwrap();

        let first = install_at(&image, &fixture.paths, || image.still_matches_path()).unwrap();
        assert!(first.installed);
        assert!(!first.needs_refresh);
        assert_eq!(fs::read(&fixture.paths.icon).unwrap(), ICON_BYTES);
        let desktop = fs::read(&fixture.paths.desktop).unwrap();
        let second = install_at(&image, &fixture.paths, || image.still_matches_path()).unwrap();
        assert!(second.installed);
        assert_eq!(fs::read(&fixture.paths.desktop).unwrap(), desktop);

        let removed = remove_at(&image, &fixture.paths).unwrap();
        assert!(!removed.installed);
        assert!(!fixture.paths.desktop.exists());
        assert!(!fixture.paths.icon.exists());
        assert_eq!(fs::read(package_entry).unwrap(), b"package-managed entry");
    }

    #[test]
    fn refresh_targets_new_location_and_replacement_at_same_path() {
        let mut fixture = Fixture::new("old image.AppImage");
        let old = fixture.identity();
        install_at(&old, &fixture.paths, || old.still_matches_path()).unwrap();
        let old_entry = fs::read_to_string(&fixture.paths.desktop).unwrap();

        fixture.image = fixture.root.join("new $% image.AppImage");
        Fixture::write_image(&fixture.image);
        let moved = fixture.identity();
        assert!(status_at(&moved, &fixture.paths).unwrap().needs_refresh);
        install_at(&moved, &fixture.paths, || moved.still_matches_path()).unwrap();
        let refreshed = fs::read_to_string(&fixture.paths.desktop).unwrap();
        assert_ne!(old_entry, refreshed);
        assert!(refreshed.contains(&exec_argument(moved.path().to_str().unwrap()).unwrap()));

        let replacement = fixture.root.join("replacement");
        Fixture::write_image(&replacement);
        fs::rename(&replacement, &fixture.image).unwrap();
        let new_identity = fixture.identity();
        assert_ne!(moved, new_identity);
        assert!(
            !status_at(&new_identity, &fixture.paths)
                .unwrap()
                .needs_refresh
        );
        assert_eq!(
            fs::read_to_string(&fixture.paths.desktop).unwrap(),
            refreshed
        );
    }

    #[test]
    fn unrelated_or_symlinked_destinations_are_never_overwritten() {
        let fixture = Fixture::new("Cassette.AppImage");
        let image = fixture.identity();
        fs::create_dir_all(fixture.paths.desktop.parent().unwrap()).unwrap();
        fs::write(&fixture.paths.desktop, b"[Desktop Entry]\nName=Unrelated\n").unwrap();
        assert!(install_at(&image, &fixture.paths, || true).is_err());
        assert!(remove_at(&image, &fixture.paths).is_err());
        assert_eq!(
            fs::read(&fixture.paths.desktop).unwrap(),
            b"[Desktop Entry]\nName=Unrelated\n"
        );

        fs::remove_file(&fixture.paths.desktop).unwrap();
        let unrelated = fixture.root.join("unrelated.desktop");
        fs::write(&unrelated, b"unrelated").unwrap();
        symlink(&unrelated, &fixture.paths.desktop).unwrap();
        assert!(install_at(&image, &fixture.paths, || true).is_err());
        assert_eq!(fs::read(unrelated).unwrap(), b"unrelated");
    }

    #[test]
    fn conflicting_icon_or_changed_image_fails_without_success() {
        let fixture = Fixture::new("Cassette.AppImage");
        let image = fixture.identity();
        fs::create_dir_all(fixture.paths.icon.parent().unwrap()).unwrap();
        fs::write(&fixture.paths.icon, b"unrelated icon").unwrap();
        assert!(install_at(&image, &fixture.paths, || true).is_err());
        assert!(!fixture.paths.desktop.exists());
        assert_eq!(fs::read(&fixture.paths.icon).unwrap(), b"unrelated icon");

        fs::remove_file(&fixture.paths.icon).unwrap();
        fs::write(&fixture.paths.icon, ICON_BYTES).unwrap();
        assert!(install_at(&image, &fixture.paths, || true).is_err());
        assert!(!fixture.paths.desktop.exists());
        fs::remove_file(&fixture.paths.icon).unwrap();
        assert!(install_at(&image, &fixture.paths, || false).is_err());
        assert!(!fixture.paths.desktop.exists());
        assert!(!fixture.paths.icon.exists());
    }
}
