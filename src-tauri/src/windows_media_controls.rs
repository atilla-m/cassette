use crate::{image_extension, MediaTrack, MAX_FOLDER_COVER_BYTES};
use std::fs::File;
use std::io::{Cursor, Read};
use std::path::Path;
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, Manager};
use windows::core::{factory, HSTRING};
use windows::Foundation::TypedEventHandler;
use windows::Media::{
    MediaPlaybackStatus, MediaPlaybackType, SystemMediaTransportControls,
    SystemMediaTransportControlsButton as Button,
    SystemMediaTransportControlsButtonPressedEventArgs,
};
use windows::Storage::Streams::{IRandomAccessStream, RandomAccessStreamReference};
use windows::Win32::System::WinRT::{
    CreateRandomAccessStreamOverStream, ISystemMediaTransportControlsInterop, BSOS_DEFAULT,
};
use windows::Win32::UI::Shell::SHCreateMemStream;

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
        let thumbnail = track.and_then(track_thumbnail);
        let mut status = self
            .status
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let next = track.map_or(MediaPlaybackStatus::Closed, |_| playback_status(is_playing));
        self.controls.SetPlaybackStatus(next)?;
        self.controls.SetIsEnabled(track.is_some())?;
        *status = next;

        let updater = self.controls.DisplayUpdater()?;
        updater.ClearAll()?;
        if let Some(track) = track {
            updater.SetType(MediaPlaybackType::Music)?;
            let music = updater.MusicProperties()?;
            music.SetTitle(&HSTRING::from(track.title.as_str()))?;
            music.SetArtist(&HSTRING::from(track.artist.as_deref().unwrap_or_default()))?;
            music.SetAlbumTitle(&HSTRING::from(track.album.as_deref().unwrap_or_default()))?;
            if let Some(thumbnail) = thumbnail {
                // Artwork can fail independently of the song's text and controls.
                let _ = updater.SetThumbnail(&thumbnail);
            }
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

fn track_thumbnail(track: &MediaTrack) -> Option<RandomAccessStreamReference> {
    track
        .custom_art_path
        .as_deref()
        .into_iter()
        .chain(track.art_path.as_deref())
        .find_map(|path| cover_thumbnail(Path::new(path)))
}

fn cover_thumbnail(path: &Path) -> Option<RandomAccessStreamReference> {
    let mut bytes = Vec::new();
    File::open(path)
        .ok()?
        .take(MAX_FOLDER_COVER_BYTES + 1)
        .read_to_end(&mut bytes)
        .ok()?;
    if bytes.len() as u64 > MAX_FOLDER_COVER_BYTES {
        return None;
    }
    if image_extension(&bytes)? == "webp" {
        // Supply a PNG so the panel doesn't depend on a system WebP codec.
        let mut reader =
            image::ImageReader::with_format(Cursor::new(bytes), image::ImageFormat::WebP);
        let mut limits = image::Limits::default();
        limits.max_image_width = Some(6000);
        limits.max_image_height = Some(6000);
        limits.max_alloc = Some(192 * 1024 * 1024);
        reader.limits(limits);
        let image = reader.decode().ok()?.thumbnail(512, 512);
        let mut png = Cursor::new(Vec::new());
        image.write_to(&mut png, image::ImageFormat::Png).ok()?;
        bytes = png.into_inner();
    }

    // SAFETY: SHCreateMemStream copies the bounded byte slice. The WinRT wrapper
    // retains the stream, so Windows can read the cover after this call returns.
    let stream = unsafe { SHCreateMemStream(Some(&bytes)) }?;
    let random_access: IRandomAccessStream =
        unsafe { CreateRandomAccessStreamOverStream(&stream, BSOS_DEFAULT) }.ok()?;
    RandomAccessStreamReference::CreateFromStream(&random_access).ok()
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

    #[test]
    fn cover_thumbnail_can_be_read_after_the_source_file_is_gone() {
        use windows::Storage::Streams::{Buffer, DataReader, InputStreamOptions};
        use windows::Win32::System::WinRT::{RoInitialize, RoUninitialize, RO_INIT_MULTITHREADED};

        // The Rust test thread has no Windows Runtime apartment yet.
        unsafe { RoInitialize(RO_INIT_MULTITHREADED) }.unwrap();
        struct RuntimeApartment;
        impl Drop for RuntimeApartment {
            fn drop(&mut self) {
                unsafe { RoUninitialize() };
            }
        }
        let _apartment = RuntimeApartment;

        let directory = tempfile::tempdir().unwrap();
        let cover = directory.path().join("cover.webp");
        image::RgbaImage::from_pixel(32, 32, image::Rgba([39, 87, 181, 255]))
            .save(&cover)
            .unwrap();
        let art = track_thumbnail(&MediaTrack {
            custom_art_path: Some(
                directory
                    .path()
                    .join("missing.jpg")
                    .to_string_lossy()
                    .into_owned(),
            ),
            art_path: Some(cover.to_string_lossy().into_owned()),
            ..MediaTrack::default()
        })
        .expect("fall back to scanned art");
        std::fs::remove_file(cover).unwrap();
        let stream = art.OpenReadAsync().unwrap().get().unwrap();
        let size = u32::try_from(stream.Size().unwrap()).unwrap();
        let buffer = Buffer::Create(size).unwrap();
        let loaded = stream
            .ReadAsync(&buffer, size, InputStreamOptions::None)
            .unwrap()
            .get()
            .unwrap();
        let mut bytes = vec![0; size as usize];
        DataReader::FromBuffer(&loaded)
            .unwrap()
            .ReadBytes(&mut bytes)
            .unwrap();
        assert_eq!(
            image::guess_format(&bytes).unwrap(),
            image::ImageFormat::Png
        );
        let decoded = image::load_from_memory(&bytes).unwrap().to_rgba8();
        assert_eq!(decoded.get_pixel(0, 0).0, [39, 87, 181, 255]);
        assert!(track_thumbnail(&MediaTrack::default()).is_none());
    }
}
