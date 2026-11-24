use crate::MediaTrack;
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, Manager};
use windows::core::{factory, HSTRING};
use windows::Foundation::TypedEventHandler;
use windows::Media::{
    MediaPlaybackStatus, MediaPlaybackType, SystemMediaTransportControls,
    SystemMediaTransportControlsButton as Button,
    SystemMediaTransportControlsButtonPressedEventArgs,
};
use windows::Win32::System::WinRT::ISystemMediaTransportControlsInterop;

pub(super) struct WindowsMediaControls(Option<MediaSession>);

struct MediaSession {
    controls: SystemMediaTransportControls,
    button_token: i64,
    status: Mutex<MediaPlaybackStatus>,
}

impl WindowsMediaControls {
    pub(super) fn new(app: AppHandle) -> Self {
        match MediaSession::new(app) {
            Ok(session) => Self(Some(session)),
            Err(error) => {
                eprintln!("Windows media controls are unavailable: {error}");
                Self(None)
            }
        }
    }

    pub(super) fn update_track(&self, track: Option<MediaTrack>, is_playing: bool) {
        let Some(session) = &self.0 else { return };
        if let Err(error) = session.update_track(track.as_ref(), is_playing) {
            eprintln!("Could not update Windows media controls: {error}");
        }
    }

    pub(super) fn update_playback(&self, is_playing: bool, _position_seconds: u64, _volume: f64) {
        let Some(session) = &self.0 else { return };
        let Ok(mut status) = session.status.lock() else {
            return;
        };
        let next = playback_status(is_playing);
        if *status != MediaPlaybackStatus::Closed
            && *status != next
            && session.controls.SetPlaybackStatus(next).is_ok()
        {
            *status = next;
        }
    }
}

impl MediaSession {
    fn new(app: AppHandle) -> Result<Self, Box<dyn std::error::Error>> {
        let window = app
            .get_webview_window("main")
            .ok_or("Cassette's main window is unavailable")?;
        let hwnd = window.hwnd()?;
        let interop =
            factory::<SystemMediaTransportControls, ISystemMediaTransportControlsInterop>()?;
        // SAFETY: hwnd belongs to this process and refers to Cassette's live top-level window.
        let controls: SystemMediaTransportControls = unsafe { interop.GetForWindow(hwnd)? };
        controls.SetIsEnabled(false)?;
        controls.SetIsPlayEnabled(true)?;
        controls.SetIsPauseEnabled(true)?;
        controls.SetIsNextEnabled(true)?;
        controls.SetIsPreviousEnabled(true)?;
        controls.SetIsStopEnabled(true)?;

        let handler = TypedEventHandler::<
            SystemMediaTransportControls,
            SystemMediaTransportControlsButtonPressedEventArgs,
        >::new(move |_, args| {
            let Some(args) = args.as_ref() else {
                return Ok(());
            };
            if let Some(event) = button_event(args.Button()?) {
                let _ = app.emit_to("main", event, ());
            }
            Ok(())
        });
        let button_token = controls.ButtonPressed(&handler)?;
        Ok(Self {
            controls,
            button_token,
            status: Mutex::new(MediaPlaybackStatus::Closed),
        })
    }

    fn update_track(
        &self,
        track: Option<&MediaTrack>,
        is_playing: bool,
    ) -> windows::core::Result<()> {
        let mut status = self
            .status
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let next = track.map_or(MediaPlaybackStatus::Closed, |_| playback_status(is_playing));
        self.controls.SetPlaybackStatus(next)?;
        self.controls.SetIsEnabled(track.is_some())?;
        *status = next;

        let updater = self.controls.DisplayUpdater()?;
        if let Some(track) = track {
            updater.SetType(MediaPlaybackType::Music)?;
            let music = updater.MusicProperties()?;
            music.SetTitle(&HSTRING::from(track.title.as_str()))?;
            music.SetArtist(&HSTRING::from(track.artist.as_deref().unwrap_or_default()))?;
            music.SetAlbumTitle(&HSTRING::from(track.album.as_deref().unwrap_or_default()))?;
        } else {
            updater.ClearAll()?;
        }
        updater.Update()
    }
}

impl Drop for MediaSession {
    fn drop(&mut self) {
        let _ = self.controls.SetIsEnabled(false);
        let _ = self.controls.RemoveButtonPressed(self.button_token);
    }
}

fn playback_status(is_playing: bool) -> MediaPlaybackStatus {
    if is_playing {
        MediaPlaybackStatus::Playing
    } else {
        MediaPlaybackStatus::Paused
    }
}

fn button_event(button: Button) -> Option<&'static str> {
    match button {
        Button::Play => Some("media-play"),
        Button::Pause => Some("media-pause"),
        Button::Next => Some("media-next"),
        Button::Previous => Some("media-previous"),
        Button::Stop => Some("media-stop"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn media_buttons_use_the_existing_player_actions() {
        assert_eq!(button_event(Button::Play), Some("media-play"));
        assert_eq!(button_event(Button::Pause), Some("media-pause"));
        assert_eq!(button_event(Button::Next), Some("media-next"));
        assert_eq!(button_event(Button::Previous), Some("media-previous"));
        assert_eq!(button_event(Button::Stop), Some("media-stop"));
        assert_eq!(button_event(Button::Record), None);
    }
}
