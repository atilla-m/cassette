use super::*;
use std::cell::RefCell;
use std::os::unix::fs::{symlink, PermissionsExt};

struct SyntheticDisc {
    root: PathBuf,
    output: PathBuf,
    reader: PathBuf,
}

impl SyntheticDisc {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "Cassette synthetic CD テスト-{}-{}",
            std::process::id(),
            unique_timestamp_nanos()
        ));
        let output = root.join("output");
        fs::create_dir_all(&output).unwrap();
        let reader = root.join("reader.py");
        fs::write(
            &reader,
            r#"#!/usr/bin/env python3
import pathlib, sys, wave
reader = pathlib.Path(__file__)
if sys.argv[1] == '--version':
    print('Cassette synthetic reader; no hardware')
elif sys.argv[1] == '-Q':
    print('\n'.join(f'{number}.  441  [00:00.10]' for number in range(1, 7)))
else:
    number, destination = int(sys.argv[1]), pathlib.Path(sys.argv[2])
    with reader.with_suffix('.reads').open('a') as log:
        log.write(f'{number}\n')
    with wave.open(str(destination), 'wb') as output:
        output.setparams((2, 2, 44100, 0, 'NONE', 'not compressed'))
        output.writeframes(bytes(4410 * 4))
    collision = reader.with_suffix('.collision')
    if number == 2 and collision.exists():
        (destination.parent / collision.read_text()).write_bytes(b'Existing destination survives')
"#,
        )
        .unwrap();
        fs::set_permissions(&reader, fs::Permissions::from_mode(0o700)).unwrap();
        Self {
            root,
            output,
            reader,
        }
    }

    fn rip(&self, metadata: Option<CdRipMetadata>) -> CdRipResult {
        rip_cd_to_flac_with_reader(
            self.output.to_string_lossy().into_owned(),
            metadata,
            &self.reader,
            |_, _| {},
        )
        .expect("run real job with synthetic disc reader and installed FLAC encoder")
    }
}

impl Drop for SyntheticDisc {
    fn drop(&mut self) {
        if !std::thread::panicking() {
            fs::remove_dir_all(&self.root).unwrap();
        }
    }
}

// Deserialize the frontend's camelCase request metadata, rather than constructing
// a different test-only representation of the native job.
fn request_metadata() -> CdRipMetadata {
    serde_json::from_value(serde_json::json!({
        "albumArtist": "Fixture Artist", "albumTitle": "Fixture Album",
        "year": "2026", "genre": "Test Genre", "discNumber": 2, "cover": null,
        "tracks": [
            {"number":1,"title":"Ark","artist":"","discNumber":null},
            {"number":2,"title":"Ark","artist":"Track Artist","discNumber":3},
            {"number":3,"title":"夜明け – 東京","artist":"","discNumber":null},
            {"number":4,"title":" \t ","artist":"","discNumber":null},
            {"number":5,"title":"../夜/道\\彼方..","artist":"","discNumber":null},
            {"number":6,"title":"...","artist":"","discNumber":null}
        ]
    }))
    .unwrap()
}

#[test]
fn submitted_titles_reach_real_filenames_flac_tags_events_and_scanner() {
    let disc = SyntheticDisc::new();
    let events = RefCell::new(Vec::new());
    let result = rip_cd_to_flac_with_reader(
        disc.output.to_string_lossy().into_owned(),
        Some(request_metadata()),
        &disc.reader,
        |event, payload| events.borrow_mut().push((event.to_owned(), payload)),
    )
    .unwrap();
    let expected = [
        ("01 - Ark.flac", "Ark"),
        ("02 - Ark.flac", "Ark"),
        ("03 - 夜明け – 東京.flac", "夜明け – 東京"),
        ("04 - Track 04.flac", "Track 04"),
        ("05 - -夜-道-彼方.flac", "../夜/道\\彼方.."),
        ("06 - Track 06.flac", "..."),
    ];
    assert_eq!(result.tracks.len(), expected.len());
    for (track, (filename, title)) in result.tracks.iter().zip(expected) {
        assert_eq!(track.status.as_deref(), Some("done"));
        if track.number >= 5 {
            assert!(track.warning.as_ref().unwrap().contains("Filename"));
        } else {
            assert_eq!(track.warning, None);
        }
        assert_eq!(track.output_filename.as_deref(), Some(filename));
        let path = Path::new(&result.output_folder).join(filename);
        assert_eq!(track.output_path.as_deref(), path.to_str());
        assert!(path.is_file());
        let tagged = lofty::read_from_path(&path).unwrap();
        let tag = tagged.primary_tag().unwrap();
        assert_eq!(tag.title().as_deref(), Some(title));
        assert_eq!(tag.album().as_deref(), Some("Fixture Album"));
        assert_eq!(tag.get_string(ItemKey::AlbumArtist), Some("Fixture Artist"));
        assert_eq!(
            tag.artist().as_deref(),
            Some(if track.number == 2 {
                "Track Artist"
            } else {
                "Fixture Artist"
            })
        );
        assert_eq!(tag.track(), Some(track.number));
        assert_eq!(tag.track_total(), Some(6));
        assert_eq!(tag.disk(), Some(if track.number == 2 { 3 } else { 2 }));
        assert_eq!(tag.genre().as_deref(), Some("Test Genre"));
        assert_eq!(tag.get_string(ItemKey::RecordingDate), Some("2026"));
        let (scanned, _) = track_from_path(path.clone(), 1_700_000_000).unwrap();
        assert_eq!(scanned.title, title);
        assert_eq!(scanned.file_path, path.to_string_lossy());
        for event_name in ["cd-rip-track-started", "cd-rip-track-finished"] {
            assert!(events
                .borrow()
                .iter()
                .any(|(event, payload)| event == event_name
                    && payload.track_number == Some(track.number)
                    && payload.output_filename.as_deref() == Some(filename)
                    && payload.output_path.as_deref() == path.to_str()));
        }
    }
    let decoded = Command::new("flac")
        .args([
            "--decode",
            "--stdout",
            "--silent",
            "--force-raw-format",
            "--endian=little",
            "--sign=signed",
        ])
        .arg(Path::new(&result.output_folder).join("02 - Ark.flac"))
        .output()
        .unwrap();
    assert!(decoded.status.success());
    assert_eq!(
        decoded.stdout,
        vec![0; 4410 * 4],
        "tag writing preserves the synthetic audio"
    );
}

#[test]
fn missing_metadata_still_writes_deterministic_title_tags() {
    let disc = SyntheticDisc::new();
    let result = disc.rip(None);
    for track in result.tracks {
        assert_eq!(track.status.as_deref(), Some("done"));
        let path = Path::new(track.output_path.as_ref().unwrap());
        let tagged = lofty::read_from_path(path).unwrap();
        assert_eq!(
            tagged.primary_tag().unwrap().title().as_deref(),
            Some(format!("Track {:02}", track.number).as_str())
        );
    }
}

#[test]
fn existing_rip_folder_and_repeated_jobs_keep_existing_files() {
    let disc = SyntheticDisc::new();
    let name = cd_rip_folder_name(Some(&request_metadata())).0;
    let existing = disc.output.join(&name);
    fs::create_dir(&existing).unwrap();
    let original = existing.join("02 - Ark.flac");
    fs::write(&original, b"Existing unrelated file").unwrap();
    let first = disc.rip(Some(request_metadata()));
    let second = disc.rip(Some(request_metadata()));
    assert_ne!(first.output_folder, second.output_folder);
    assert!(first.output_folder.ends_with(&format!("{name} (2)")));
    assert!(second.output_folder.ends_with(&format!("{name} (3)")));
    assert_eq!(fs::read(original).unwrap(), b"Existing unrelated file");
}

#[test]
fn existing_destination_fails_before_reading_and_reports_actual_path() {
    let disc = SyntheticDisc::new();
    let result = rip_cd_to_flac_with_reader(
        disc.output.to_string_lossy().into_owned(),
        Some(request_metadata()),
        &disc.reader,
        |event, payload| {
            if event == "cd-rip-track-started" && payload.track_number == Some(2) {
                fs::write(
                    payload.output_path.unwrap(),
                    b"Existing unrelated destination",
                )
                .unwrap();
            }
        },
    )
    .unwrap();
    let track = &result.tracks[1];
    assert_eq!(track.status.as_deref(), Some("error"));
    assert!(track
        .error
        .as_ref()
        .unwrap()
        .contains("will not be overwritten"));
    assert_eq!(
        fs::read(track.output_path.as_ref().unwrap()).unwrap(),
        b"Existing unrelated destination"
    );
    assert!(!fs::read_to_string(disc.reader.with_extension("reads"))
        .unwrap()
        .lines()
        .any(|line| line == "2"));
    let wav = disc.root.join("existing.wav");
    fs::write(&wav, b"Existing temporary destination").unwrap();
    assert!(ensure_cd_destination_absent(&wav).is_err());
    let link = disc.root.join("broken.flac");
    symlink(disc.root.join("missing"), &link).unwrap();
    assert!(ensure_cd_destination_absent(&link).is_err());
}

#[test]
fn encoder_refuses_a_destination_that_appears_after_preflight() {
    let disc = SyntheticDisc::new();
    fs::write(disc.reader.with_extension("collision"), "02 - Ark.flac").unwrap();
    let result = disc.rip(Some(request_metadata()));
    let track = &result.tracks[1];
    assert_eq!(track.status.as_deref(), Some("error"));
    assert!(track
        .error
        .as_ref()
        .unwrap()
        .contains("FLAC encoding failed"));
    assert_eq!(
        fs::read(track.output_path.as_ref().unwrap()).unwrap(),
        b"Existing destination survives"
    );
}

#[test]
fn rip_folder_reservation_is_atomic_and_does_not_follow_existing_links() {
    let disc = SyntheticDisc::new();
    symlink(disc.root.join("missing"), disc.output.join("same name")).unwrap();
    let barrier = std::sync::Barrier::new(2);
    let paths = std::thread::scope(|scope| {
        let left = scope.spawn(|| {
            barrier.wait();
            create_cd_rip_folder(&disc.output, "same name").unwrap()
        });
        let right = scope.spawn(|| {
            barrier.wait();
            create_cd_rip_folder(&disc.output, "same name").unwrap()
        });
        [left.join().unwrap(), right.join().unwrap()]
    });
    assert_ne!(paths[0], paths[1]);
    assert!(paths.iter().all(|path| path.is_dir()));
    assert!(fs::symlink_metadata(disc.output.join("same name"))
        .unwrap()
        .file_type()
        .is_symlink());
}
