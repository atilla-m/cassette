#[cfg(target_os = "linux")]
mod platform {
    use std::collections::HashMap;
    use std::path::Path;

    use tauri::{AppHandle, Emitter};
    use zbus::blocking::Connection;
    use zbus::object_server::SignalEmitter;
    use zbus::zvariant::{ObjectPath, OwnedValue, Value};
    use zbus::{block_on, interface};

    const MPRIS_BUS_NAME: &str = "org.mpris.MediaPlayer2.Cassette";
    const MPRIS_PATH: &str = "/org/mpris/MediaPlayer2";

    #[derive(Debug, Clone, Default)]
    pub struct MprisTrack {
        pub title: String,
        pub artist: Option<String>,
        pub album: Option<String>,
        pub duration_seconds: Option<u64>,
        pub art_path: Option<String>,
    }

    #[derive(Debug, Clone)]
    struct MprisSnapshot {
        playback_status: String,
        metadata: HashMap<String, OwnedValue>,
        position_microseconds: i64,
        volume: f64,
    }

    impl Default for MprisSnapshot {
        fn default() -> Self {
            Self {
                playback_status: "Stopped".to_owned(),
                metadata: empty_metadata(),
                position_microseconds: 0,
                volume: 1.0,
            }
        }
    }

    pub struct MprisState {
        connection: Option<Connection>,
    }

    impl MprisState {
        pub fn new(app: AppHandle) -> Self {
            let root = RootInterface { app: app.clone() };
            let player = PlayerInterface {
                app,
                snapshot: MprisSnapshot::default(),
            };

            match zbus::blocking::connection::Builder::session()
                .and_then(|builder| builder.serve_at(MPRIS_PATH, root))
                .and_then(|builder| builder.serve_at(MPRIS_PATH, player))
                .and_then(|builder| builder.name(MPRIS_BUS_NAME))
                .and_then(|builder| builder.build())
            {
                Ok(connection) => Self {
                    connection: Some(connection),
                },
                Err(error) => {
                    eprintln!("MPRIS is unavailable: {error}");
                    Self { connection: None }
                }
            }
        }

        pub fn update_track(&self, track: Option<MprisTrack>, is_playing: bool) {
            let Some(connection) = &self.connection else {
                return;
            };
            let Ok(interface_ref) = connection
                .object_server()
                .interface::<_, PlayerInterface>(MPRIS_PATH)
            else {
                return;
            };

            {
                let mut interface = interface_ref.get_mut();
                interface.snapshot.metadata = track
                    .as_ref()
                    .map(metadata_for_track)
                    .unwrap_or_else(empty_metadata);
                interface.snapshot.position_microseconds = 0;
                interface.snapshot.playback_status = playback_status(is_playing).to_owned();
                let emitter = interface_ref.signal_emitter();
                let _ = block_on(interface.metadata_changed(emitter));
                let _ = block_on(interface.playback_status_changed(emitter));
                let _ = block_on(interface.position_changed(emitter));
            }
        }

        pub fn update_playback(&self, is_playing: bool, position_seconds: u64, volume: f64) {
            let Some(connection) = &self.connection else {
                return;
            };
            let Ok(interface_ref) = connection
                .object_server()
                .interface::<_, PlayerInterface>(MPRIS_PATH)
            else {
                return;
            };
            let next_status = playback_status(is_playing);
            let next_position = seconds_to_microseconds(position_seconds);

            {
                let mut interface = interface_ref.get_mut();
                let should_emit_status = interface.snapshot.playback_status != next_status;
                interface.snapshot.playback_status = next_status.to_owned();
                interface.snapshot.position_microseconds = next_position;
                interface.snapshot.volume = volume.clamp(0.0, 1.0);
                let emitter = interface_ref.signal_emitter();
                if should_emit_status {
                    let _ = block_on(interface.playback_status_changed(emitter));
                }
                let _ = block_on(interface.position_changed(emitter));
                let _ = block_on(interface.volume_changed(emitter));
            }
        }
    }

    struct RootInterface {
        app: AppHandle,
    }

    #[interface(name = "org.mpris.MediaPlayer2")]
    impl RootInterface {
        fn raise(&self) {
            let _ = self.app.emit("mpris-raise", ());
        }

        fn quit(&self) {}

        #[zbus(property)]
        fn can_quit(&self) -> bool {
            false
        }

        #[zbus(property)]
        fn fullscreen(&self) -> bool {
            false
        }

        #[zbus(property)]
        fn can_set_fullscreen(&self) -> bool {
            false
        }

        #[zbus(property)]
        fn can_raise(&self) -> bool {
            true
        }

        #[zbus(property)]
        fn has_track_list(&self) -> bool {
            false
        }

        #[zbus(property)]
        fn identity(&self) -> &str {
            "Cassette"
        }

        #[zbus(property)]
        fn desktop_entry(&self) -> &str {
            "cassette"
        }

        #[zbus(property)]
        fn supported_uri_schemes(&self) -> Vec<&str> {
            vec!["file"]
        }

        #[zbus(property)]
        fn supported_mime_types(&self) -> Vec<&str> {
            Vec::new()
        }
    }

    struct PlayerInterface {
        app: AppHandle,
        snapshot: MprisSnapshot,
    }

    #[interface(name = "org.mpris.MediaPlayer2.Player")]
    impl PlayerInterface {
        fn next(&self) {
            let _ = self.app.emit("mpris-next", ());
        }

        fn previous(&self) {
            let _ = self.app.emit("mpris-previous", ());
        }

        fn pause(&self) {
            let _ = self.app.emit("mpris-pause", ());
        }

        fn play_pause(&self) {
            let _ = self.app.emit("mpris-play-pause", ());
        }

        fn stop(&self) {
            let _ = self.app.emit("mpris-stop", ());
        }

        fn play(&self) {
            let _ = self.app.emit("mpris-play", ());
        }

        fn seek(&self, _offset: i64) {}

        fn set_position(&self, _track_id: ObjectPath<'_>, position: i64) {
            let seconds = (position / 1_000_000).max(0) as u64;
            let _ = self.app.emit("mpris-seek", seconds);
        }

        fn open_uri(&self, _uri: &str) {}

        #[zbus(property)]
        fn playback_status(&self) -> &str {
            &self.snapshot.playback_status
        }

        #[zbus(property)]
        fn loop_status(&self) -> &str {
            "None"
        }

        #[zbus(property)]
        fn rate(&self) -> f64 {
            1.0
        }

        #[zbus(property)]
        fn shuffle(&self) -> bool {
            false
        }

        #[zbus(property)]
        fn metadata(&self) -> HashMap<String, OwnedValue> {
            self.snapshot.metadata.clone()
        }

        #[zbus(property)]
        fn volume(&self) -> f64 {
            self.snapshot.volume
        }

        #[zbus(property)]
        fn position(&self) -> i64 {
            self.snapshot.position_microseconds
        }

        #[zbus(property)]
        fn minimum_rate(&self) -> f64 {
            1.0
        }

        #[zbus(property)]
        fn maximum_rate(&self) -> f64 {
            1.0
        }

        #[zbus(property)]
        fn can_go_next(&self) -> bool {
            true
        }

        #[zbus(property)]
        fn can_go_previous(&self) -> bool {
            true
        }

        #[zbus(property)]
        fn can_play(&self) -> bool {
            true
        }

        #[zbus(property)]
        fn can_pause(&self) -> bool {
            true
        }

        #[zbus(property)]
        fn can_seek(&self) -> bool {
            true
        }

        #[zbus(property)]
        fn can_control(&self) -> bool {
            true
        }

        #[zbus(signal)]
        async fn seeked(emitter: &SignalEmitter<'_>, position: i64) -> zbus::Result<()>;
    }

    fn metadata_for_track(track: &MprisTrack) -> HashMap<String, OwnedValue> {
        let mut metadata = empty_metadata();
        insert_value(&mut metadata, "xesam:title", track.title.clone());

        if let Some(artist) = track.artist.as_ref() {
            insert_value(&mut metadata, "xesam:artist", vec![artist.clone()]);
        }

        if let Some(album) = track.album.as_ref() {
            insert_value(&mut metadata, "xesam:album", album.clone());
        }

        if let Some(duration_seconds) = track.duration_seconds {
            insert_value(
                &mut metadata,
                "mpris:length",
                seconds_to_microseconds(duration_seconds),
            );
        }

        if let Some(art_path) = track.art_path.as_ref().and_then(path_to_file_uri) {
            insert_value(&mut metadata, "mpris:artUrl", art_path);
        }

        metadata
    }

    fn empty_metadata() -> HashMap<String, OwnedValue> {
        let mut metadata = HashMap::new();
        let track_id = ObjectPath::from_static_str("/org/mpris/MediaPlayer2/Track/current")
            .expect("static MPRIS track path is valid");
        insert_value(&mut metadata, "mpris:trackid", track_id);

        metadata
    }

    fn insert_value<T>(metadata: &mut HashMap<String, OwnedValue>, key: &str, value: T)
    where
        Value<'static>: From<T>,
    {
        if let Ok(value) = OwnedValue::try_from(Value::from(value)) {
            metadata.insert(key.to_owned(), value);
        }
    }

    fn playback_status(is_playing: bool) -> &'static str {
        if is_playing {
            "Playing"
        } else {
            "Paused"
        }
    }

    fn seconds_to_microseconds(seconds: u64) -> i64 {
        i64::try_from(seconds.saturating_mul(1_000_000)).unwrap_or(i64::MAX)
    }

    fn path_to_file_uri(path: &String) -> Option<String> {
        gstreamer::glib::filename_to_uri(Path::new(path), None)
            .ok()
            .map(|uri| uri.to_string())
    }
}

#[cfg(not(target_os = "linux"))]
mod platform {
    use tauri::AppHandle;

    #[derive(Debug, Clone, Default)]
    pub struct MprisTrack {
        pub title: String,
        pub artist: Option<String>,
        pub album: Option<String>,
        pub duration_seconds: Option<u64>,
        pub art_path: Option<String>,
    }

    pub struct MprisState;

    impl MprisState {
        pub fn new(_app: AppHandle) -> Self {
            Self
        }

        pub fn update_track(&self, _track: Option<MprisTrack>, _is_playing: bool) {}

        pub fn update_playback(&self, _is_playing: bool, _position_seconds: u64, _volume: f64) {}
    }
}

pub use platform::{MprisState, MprisTrack};
