use super::{Progress, UpdateInfo, UpdateSession};
use base64::{engine::general_purpose::STANDARD, Engine};
use minisign_verify::Signature;
use reqwest::Url;
use semver::Version;
use serde::Deserialize;
use std::time::Duration;
use tauri::{AppHandle, Manager};
use tauri_plugin_updater::UpdaterExt;

const RELEASES_API: &str = "https://api.github.com/repos/atilla-m/cassette/releases";
const RELEASE_DOWNLOAD: &str = "https://github.com/atilla-m/cassette/releases/download";
const MANIFEST: &str = "windows-update.json";

#[derive(Deserialize)]
struct GithubRelease {
    tag_name: String,
    draft: bool,
    prerelease: bool,
    assets: Vec<ReleaseAsset>,
}

#[derive(Deserialize)]
struct ReleaseAsset {
    name: String,
}

fn installer_name(version: &Version) -> String {
    format!("Cassette_{version}_x64-setup.exe")
}

impl GithubRelease {
    fn update_version(&self) -> Option<Version> {
        if self.draft || !self.prerelease {
            return None;
        }
        let version = Version::parse(self.tag_name.strip_prefix("windows-v")?).ok()?;
        if !version.pre.as_str().starts_with("beta.") || !version.build.is_empty() {
            return None;
        }
        let installer = installer_name(&version);
        let signature = format!("{installer}.sig");
        [MANIFEST, &installer, &signature]
            .iter()
            .all(|name| self.assets.iter().any(|asset| asset.name == *name))
            .then_some(version)
    }
}

fn signature_matches_filename(signature: &str, installer: &str) -> bool {
    if signature.len() > 4096 {
        return false;
    }
    STANDARD.decode(signature.trim()).ok().is_some_and(|bytes| {
        std::str::from_utf8(&bytes)
            .ok()
            .and_then(|signature| Signature::decode(signature).ok())
            .is_some_and(|signature| {
                let expected = format!("file:{installer}");
                signature.trusted_comment().split('\t').any(|field| field == expected)
            })
    })
}

async fn latest_beta(current: &Version) -> Result<Option<Version>, reqwest::Error> {
    let client = reqwest::Client::builder()
        .https_only(true)
        .timeout(Duration::from_secs(15))
        .user_agent("Cassette Windows Updater")
        .build()?;
    let mut latest = None;
    let mut page = 1u32;
    loop {
        let releases: Vec<GithubRelease> = client
            .get(RELEASES_API)
            .query(&[("per_page", 100), ("page", page)])
            .header("Accept", "application/vnd.github+json")
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        if let Some(candidate) = releases
            .iter()
            .filter_map(GithubRelease::update_version)
            .filter(|version| version > current)
            .max()
        {
            latest = latest.max(Some(candidate));
        }
        if releases.len() < 100 {
            return Ok(latest);
        }
        page += 1;
    }
}

pub(super) async fn check(app: AppHandle) -> Result<Option<UpdateInfo>, String> {
    let session = app.state::<UpdateSession>();
    let _busy = session.enter()?;
    if !cfg!(target_arch = "x86_64") {
        return Err("This Windows architecture does not support in-app updates.".into());
    }
    let Some(version) = latest_beta(&app.package_info().version)
        .await
        .map_err(|_| "Could not reach the Windows beta channel. Check your connection and retry.")?
    else {
        *session.pending_windows.lock().map_err(|_| "Updater state unavailable.")? = None;
        return Ok(None);
    };
    let release = format!("{RELEASE_DOWNLOAD}/windows-v{version}");
    let endpoint = Url::parse(&format!("{release}/{MANIFEST}"))
        .map_err(|_| "Invalid update endpoint.")?;
    let installer = installer_name(&version);
    let expected_payload = format!("{release}/{installer}");
    let update = app
        .updater_builder()
        .target("windows-x86_64")
        .endpoints(vec![endpoint])
        .and_then(|builder| builder.timeout(Duration::from_secs(20)).build())
        .map_err(|_| "Updater configuration is unavailable.")?
        .check()
        .await
        .map_err(|_| "Could not read the Windows update manifest.")?
        .filter(|update| {
            update.version == version.to_string()
                && update.download_url.as_str() == expected_payload
                && signature_matches_filename(&update.signature, &installer)
        })
        .ok_or("The Windows update manifest does not match the selected release.")?;
    let operation_id = uuid::Uuid::new_v4().to_string();
    let info = UpdateInfo {
        version: update.version.clone(),
        body: update.body.clone(),
        operation_id: operation_id.clone(),
    };
    *session.pending_windows.lock().map_err(|_| "Updater state unavailable.")? =
        Some((operation_id, update));
    Ok(Some(info))
}

pub(super) async fn install(
    app: AppHandle,
    operation_id: String,
    progress: tauri::ipc::Channel<Progress>,
) -> Result<(), String> {
    let session = app.state::<UpdateSession>();
    let _busy = session.enter()?;
    let mut update = session
        .pending_windows
        .lock()
        .map_err(|_| "Updater state unavailable.")?
        .as_ref()
        .filter(|(pending_id, _)| pending_id == &operation_id)
        .map(|(_, update)| update.clone())
        .ok_or("The selected update changed. Check for updates again.")?;
    update.timeout = Some(Duration::from_secs(300));
    let mut downloaded_bytes = 0;
    let installer = update
        .download(
            |count, total_bytes| {
                downloaded_bytes += count;
                let _ = progress.send(Progress {
                    downloaded_bytes,
                    total_bytes,
                    installing: false,
                });
            },
            || {},
        )
        .await
        .map_err(|_| "Update download or signature verification failed. You can retry.")?;
    let _ = progress.send(Progress {
        downloaded_bytes: installer.len(),
        total_bytes: Some(installer.len() as u64),
        installing: true,
    });
    // Tauri launches the signed NSIS installer, exits Cassette, and lets
    // the installer relaunch the app after replacement completes.
    update.install(installer).map_err(|_| "Could not start the verified Windows installer.".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn release(tag_name: &str) -> GithubRelease {
        let version = tag_name.trim_start_matches("windows-v");
        let installer = format!("Cassette_{version}_x64-setup.exe");
        GithubRelease {
            tag_name: tag_name.into(),
            draft: false,
            prerelease: true,
            assets: [MANIFEST.to_owned(), installer.clone(), format!("{installer}.sig")]
                .into_iter()
                .map(|name| ReleaseAsset { name })
                .collect(),
        }
    }

    #[test]
    fn windows_channel_requires_all_assets_and_uses_semver_order() {
        let recent = release("windows-v0.1.0-beta.10");
        let earlier = release("windows-v0.1.0-beta.9");
        assert!(recent.update_version().unwrap() > earlier.update_version().unwrap());
        assert!(release("v0.1.0-beta.10").update_version().is_none());
        assert!(release("windows-v0.1.0").update_version().is_none());
        assert!(release("windows-v0.1.0-beta.10+a").update_version().is_none());
        let mut draft = recent;
        draft.draft = true;
        assert!(draft.update_version().is_none());
        draft.draft = false;
        draft.assets.pop();
        assert!(draft.update_version().is_none());
    }
}
