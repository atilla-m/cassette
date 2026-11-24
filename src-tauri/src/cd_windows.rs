use crate::{cd_track_filename, CdDetectResult, CdRipTrack, MusicBrainzDisc};
use base64::engine::{general_purpose::GeneralPurpose, GeneralPurposeConfig};
use base64::{alphabet::Alphabet, Engine};
use gst::prelude::*;
use gstreamer as gst;
use sha1::{Digest, Sha1};
use std::ffi::c_void;
use std::fmt::Write as _;
use std::fs::File;
use std::io::{self, BufWriter, Write};
use std::mem::size_of;
use std::os::windows::fs::OpenOptionsExt;
use std::os::windows::io::AsRawHandle;
use std::path::Path;
use std::ptr::{null, null_mut};
use windows_sys::Win32::Devices::Cdrom::{
    CDDA, CDROM_TOC, IOCTL_CDROM_RAW_READ, IOCTL_CDROM_READ_TOC, RAW_READ_INFO,
};
use windows_sys::Win32::Foundation::{ERROR_NOT_READY, ERROR_NO_MEDIA_IN_DRIVE};
use windows_sys::Win32::Storage::FileSystem::{
    GetDriveTypeW, GetLogicalDrives, FILE_SHARE_READ, FILE_SHARE_WRITE,
};
use windows_sys::Win32::System::{WindowsProgramming::DRIVE_CDROM, IO::DeviceIoControl};

const FRAMES_PER_SECOND: u32 = 75;
const LEAD_IN_FRAMES: u32 = 2 * FRAMES_PER_SECOND;
const BYTES_PER_AUDIO_SECTOR: usize = 2352;
const SECTORS_PER_READ: u32 = 16;

#[derive(Debug, PartialEq, Eq)]
struct AudioTrack {
    number: u32,
    start_frame: u32,
    end_frame: u32,
}

pub(super) struct AudioDisc {
    drive: char,
    device: File,
    tracks: Vec<AudioTrack>,
}

impl AudioDisc {
    pub(super) fn detect() -> Result<Self, String> {
        find_audio_disc().map_err(|(_, error)| error)
    }

    pub(super) fn rip_tracks(&self) -> Vec<CdRipTrack> {
        self.tracks
            .iter()
            .map(|track| {
                let seconds = (track.end_frame - track.start_frame) / FRAMES_PER_SECOND;
                CdRipTrack {
                    number: track.number,
                    duration: Some(format!("{}:{:02}", seconds / 60, seconds % 60)),
                    duration_seconds: Some(seconds),
                    status: Some("pending".to_owned()),
                    output_filename: Some(cd_track_filename(track.number)),
                    error: None,
                    warning: None,
                }
            })
            .collect()
    }

    pub(super) fn musicbrainz_disc(&self) -> MusicBrainzDisc {
        musicbrainz_disc(&self.tracks)
    }

    pub(super) fn rip_track(&self, number: u32, wav: &Path, flac: &Path) -> Result<(), String> {
        let track = self
            .tracks
            .iter()
            .find(|track| track.number == number)
            .ok_or_else(|| format!("Track {number:02} is missing from the audio CD."))?;
        self.verify_disc()?;
        self.extract_wav(track, wav).map_err(|error| {
            format!(
                "Could not rip track {number:02} from {}: {error}",
                self.drive
            )
        })?;
        self.verify_disc()?;
        encode_flac(wav, flac)
    }

    fn verify_disc(&self) -> Result<(), String> {
        if read_tracks(&self.device).as_ref() != Ok(&self.tracks) {
            return Err("The audio CD changed or became unreadable during the rip.".to_owned());
        }
        Ok(())
    }

    fn extract_wav(&self, track: &AudioTrack, path: &Path) -> io::Result<()> {
        let mut wav = BufWriter::new(File::create_new(path)?);
        let sector_count = track.end_frame - track.start_frame;
        wav.write_all(&wav_header(sector_count * BYTES_PER_AUDIO_SECTOR as u32))?;
        let mut buffer = [0; SECTORS_PER_READ as usize * BYTES_PER_AUDIO_SECTOR];
        let mut sector = track.start_frame - LEAD_IN_FRAMES;
        let end = track.end_frame - LEAD_IN_FRAMES;

        while sector < end {
            let count = SECTORS_PER_READ.min(end - sector);
            let data = &mut buffer[..count as usize * BYTES_PER_AUDIO_SECTOR];
            let result = read_audio_sectors(&self.device, sector, count, data);
            if result.is_err() {
                read_audio_sectors(&self.device, sector, count, data)?;
            }
            wav.write_all(data)?;
            sector += count;
        }
        wav.flush()
    }
}

pub(super) fn detect_audio_cd() -> Result<CdDetectResult, String> {
    match find_audio_disc() {
        Ok(disc) => Ok(CdDetectResult {
            drive_found: true,
            disc_found: true,
            disc_key: Some(disc.musicbrainz_disc().id),
            tracks: disc.rip_tracks(),
            raw_output: format!("Audio CD in drive {}:", disc.drive),
            error: None,
        }),
        Err((drive_found, error)) => Ok(CdDetectResult {
            drive_found,
            disc_found: false,
            disc_key: None,
            tracks: Vec::new(),
            raw_output: String::new(),
            error: Some(error),
        }),
    }
}

fn find_audio_disc() -> Result<AudioDisc, (bool, String)> {
    // SAFETY: GetLogicalDrives takes no pointers.
    let drives = unsafe { GetLogicalDrives() };
    if drives == 0 {
        return Err((
            false,
            format!(
                "Could not list Windows drives: {}",
                io::Error::last_os_error()
            ),
        ));
    }
    let mut drive_found = false;
    let mut error = None;
    for index in 0..26 {
        if drives & (1 << index) == 0 {
            continue;
        }
        let letter = b'A' + index;
        let root = [u16::from(letter), b':' as u16, b'\\' as u16, 0];
        // SAFETY: root is a NUL-terminated drive root valid for this call.
        if unsafe { GetDriveTypeW(root.as_ptr()) } != DRIVE_CDROM {
            continue;
        }
        drive_found = true;
        let drive = char::from(letter);
        match File::options()
            .read(true)
            .share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE)
            .open(format!(r"\\.\{drive}:"))
        {
            Ok(device) => match read_tracks(&device) {
                Ok(tracks) => {
                    return Ok(AudioDisc {
                        drive,
                        device,
                        tracks,
                    })
                }
                Err(reason) => {
                    error.get_or_insert(format!("Drive {drive}: {reason}"));
                }
            },
            Err(reason) => {
                error.get_or_insert(format!("Could not open drive {drive}: {reason}"));
            }
        }
    }
    let error = error.unwrap_or_else(|| {
        if drive_found {
            "Insert an audio CD and try again.".to_owned()
        } else {
            "No CD drive was found. Connect a CD drive and try again.".to_owned()
        }
    });
    Err((drive_found, error))
}

fn read_tracks(device: &File) -> Result<Vec<AudioTrack>, String> {
    let mut toc = [0; size_of::<CDROM_TOC>()];
    match device_io(device, IOCTL_CDROM_READ_TOC, None, &mut toc) {
        Ok(length) => parse_audio_toc(&toc[..length]).map_err(str::to_owned),
        Err(error)
            if error.raw_os_error() == Some(ERROR_NOT_READY as i32)
                || error.raw_os_error() == Some(ERROR_NO_MEDIA_IN_DRIVE as i32) =>
        {
            Err("Insert an audio CD and try again.".to_owned())
        }
        Err(error) => Err(format!("Could not read the CD's track list: {error}")),
    }
}

fn parse_audio_toc(data: &[u8]) -> Result<Vec<AudioTrack>, &'static str> {
    const INVALID: &str = "The CD returned an invalid track list.";
    let [_, _, first, last, entries @ ..] = data else {
        return Err(INVALID);
    };
    if *first == 0 || first > last || *last > 99 {
        return Err(INVALID);
    }
    let count = usize::from(last - first + 1);
    let entries = entries.get(..(count + 1) * 8).ok_or(INVALID)?;
    let mut offsets = Vec::with_capacity(count + 1);
    let mut data_tracks = 0;
    for (index, entry) in entries.chunks_exact(8).enumerate() {
        let number = if index == count {
            0xaa
        } else {
            first + index as u8
        };
        if entry[2] != number || entry[6] >= 60 || entry[7] >= 75 {
            return Err(INVALID);
        }
        let frame = (u32::from(entry[5]) * 60 + u32::from(entry[6])) * FRAMES_PER_SECOND
            + u32::from(entry[7]);
        if frame < LEAD_IN_FRAMES || offsets.last().is_some_and(|&previous| frame <= previous) {
            return Err(INVALID);
        }
        offsets.push(frame);
        if index < count && entry[1] & 0x04 != 0 {
            data_tracks += 1;
        }
    }
    if data_tracks == count {
        return Err("This disc has no audio tracks.");
    }
    if data_tracks != 0 {
        return Err("Cassette for Windows currently supports audio-only CDs. This CD also contains data tracks.");
    }
    Ok(offsets
        .windows(2)
        .zip(*first..=*last)
        .map(|(pair, number)| AudioTrack {
            number: u32::from(number),
            start_frame: pair[0],
            end_frame: pair[1],
        })
        .collect())
}

fn musicbrainz_disc(tracks: &[AudioTrack]) -> MusicBrainzDisc {
    let first = tracks[0].number;
    let last = &tracks[tracks.len() - 1];
    let mut fields = [0; 100];
    fields[0] = last.end_frame;
    let mut toc = format!("{first} {} {}", last.number, last.end_frame);
    for track in tracks {
        fields[track.number as usize] = track.start_frame;
        write!(toc, " {}", track.start_frame).unwrap();
    }
    let mut hex = format!("{first:02X}{:02X}", last.number);
    for field in fields {
        write!(hex, "{field:08X}").unwrap();
    }
    // MusicBrainz uses SHA-1 with a custom Base64 alphabet and padding.
    let encoding = GeneralPurpose::new(
        &Alphabet::new("ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789._").unwrap(),
        GeneralPurposeConfig::new().with_encode_padding(false),
    );
    let id = format!("{}-", encoding.encode(Sha1::digest(hex)));
    MusicBrainzDisc { id, toc }
}

fn read_audio_sectors(device: &File, sector: u32, count: u32, output: &mut [u8]) -> io::Result<()> {
    let request = RAW_READ_INFO {
        DiskOffset: i64::from(sector) * 2048,
        SectorCount: count,
        TrackMode: CDDA,
    };
    let length = device_io(device, IOCTL_CDROM_RAW_READ, Some(&request), output)?;
    if length != output.len() {
        return Err(io::Error::new(
            io::ErrorKind::UnexpectedEof,
            "Incomplete audio sector read",
        ));
    }
    Ok(())
}

fn device_io(
    device: &File,
    command: u32,
    request: Option<&RAW_READ_INFO>,
    output: &mut [u8],
) -> io::Result<usize> {
    let (input, input_size) = request.map_or((null(), 0), |request| {
        (
            (request as *const RAW_READ_INFO).cast::<c_void>(),
            size_of::<RAW_READ_INFO>() as u32,
        )
    });
    let mut read = 0;
    // SAFETY: The file handle is live and the buffers are valid for this synchronous call.
    let ok = unsafe {
        DeviceIoControl(
            device.as_raw_handle(),
            command,
            input,
            input_size,
            output.as_mut_ptr().cast(),
            output.len() as u32,
            &mut read,
            null_mut(),
        )
    };
    if ok == 0 {
        return Err(io::Error::last_os_error());
    }
    if read as usize > output.len() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Oversized CD drive response",
        ));
    }
    Ok(read as usize)
}

fn wav_header(data_size: u32) -> [u8; 44] {
    let mut header = *b"RIFF\0\0\0\0WAVEfmt \x10\0\0\0\x01\0\x02\0\x44\xac\0\0\x10\xb1\x02\0\x04\0\x10\0data\0\0\0\0";
    header[4..8].copy_from_slice(&(data_size + 36).to_le_bytes());
    header[40..44].copy_from_slice(&data_size.to_le_bytes());
    header
}

fn encode_flac(wav: &Path, flac: &Path) -> Result<(), String> {
    gst::init().map_err(|error| format!("Could not initialize the FLAC encoder: {error}"))?;
    let pipeline = gst::Pipeline::new();
    let source = gst::ElementFactory::make("filesrc")
        .property("location", wav.to_str().ok_or("Invalid WAV path.")?)
        .build()
        .map_err(|error| format!("Could not open the temporary WAV: {error}"))?;
    let parser = gst::ElementFactory::make("wavparse")
        .build()
        .map_err(|error| format!("Could not create WAV parser: {error}"))?;
    let encoder = gst::ElementFactory::make("flacenc")
        .build()
        .map_err(|error| format!("Could not create FLAC encoder: {error}"))?;
    let sink = gst::ElementFactory::make("filesink")
        .property("location", flac.to_str().ok_or("Invalid FLAC path.")?)
        .build()
        .map_err(|error| format!("Could not open the temporary FLAC: {error}"))?;
    let elements = [&source, &parser, &encoder, &sink];
    pipeline
        .add_many(elements)
        .map_err(|error| error.to_string())?;
    gst::Element::link_many(elements).map_err(|error| error.to_string())?;
    let bus = pipeline.bus().ok_or("Could not monitor FLAC encoding.")?;
    let result = pipeline
        .set_state(gst::State::Playing)
        .map_err(|error| format!("Could not start FLAC encoding: {error}"))
        .and_then(|_| {
            let message = bus
                .timed_pop_filtered(
                    gst::ClockTime::from_seconds(300),
                    &[gst::MessageType::Eos, gst::MessageType::Error],
                )
                .ok_or("FLAC encoding timed out.")?;
            match message.view() {
                gst::MessageView::Error(error) => {
                    Err(format!("FLAC encoding failed: {}", error.error()))
                }
                _ => Ok(()),
            }
        });
    let stopped = pipeline.set_state(gst::State::Null);
    result?;
    stopped.map_err(|error| format!("Could not finish writing the FLAC: {error}"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn make_toc(first: u8, offsets: &[u32]) -> Vec<u8> {
        let num_tracks = offsets.len() as u8 - 1;
        let mut toc = vec![
            0,
            2 + offsets.len() as u8 * 8,
            first,
            first + num_tracks - 1,
        ];
        for (index, frame) in offsets.iter().copied().enumerate() {
            let number = if index == offsets.len() - 1 {
                0xaa
            } else {
                first + index as u8
            };
            toc.extend_from_slice(&[
                0,
                0x10,
                number,
                0,
                0,
                (frame / 4500) as u8,
                (frame / 75 % 60) as u8,
                (frame % 75) as u8,
            ]);
        }
        toc
    }

    #[test]
    fn toc_produces_musicbrainz_reference_disc_id() {
        let toc = make_toc(1, &[150, 15363, 32314, 46592, 63414, 80489, 95462]);
        let tracks = parse_audio_toc(&toc).unwrap();
        assert_eq!(tracks.len(), 6);
        assert_eq!(tracks[0].end_frame - tracks[0].start_frame, 15213);
        let disc = musicbrainz_disc(&tracks);
        assert_eq!(disc.id, "49HHV7Eb8UKF3aQiNmu1GR8vKTY-");
        assert_eq!(disc.toc, "1 6 95462 150 15363 32314 46592 63414 80489");
    }

    #[test]
    fn toc_rejects_data_tracks_and_invalid_offsets() {
        let mut toc = make_toc(3, &[150, 22200, 54000]);
        assert_eq!(parse_audio_toc(&toc).unwrap()[0].number, 3);
        assert!(parse_audio_toc(&toc[..toc.len() - 8]).is_err());
        toc[13] |= 0x04;
        assert!(parse_audio_toc(&toc)
            .unwrap_err()
            .contains("also contains data tracks"));
        toc[5] |= 0x04;
        assert!(parse_audio_toc(&toc)
            .unwrap_err()
            .contains("no audio tracks"));
        assert!(parse_audio_toc(&make_toc(1, &[150, 150])).is_err());
    }

    #[test]
    fn synthetic_cd_wav_round_trips_through_flac() {
        let directory = std::env::temp_dir().join(format!("cassette-rip-{}", uuid::Uuid::new_v4()));
        fs::create_dir(&directory).unwrap();
        let wav = directory.join("Test 音楽.wav");
        let flac = directory.join("Test 音楽.flac.part");
        let decoded = directory.join("decoded.pcm");
        let pcm: Vec<u8> = (0i16..5880)
            .flat_map(|sample| [sample.to_le_bytes(), (-sample).to_le_bytes()].concat())
            .collect();
        let mut input = wav_header(pcm.len() as u32).to_vec();
        input.extend_from_slice(&pcm);
        fs::write(&wav, input).unwrap();
        encode_flac(&wav, &flac).unwrap();

        let pipeline = gst::parse::launch(
            "filesrc name=source ! flacparse ! flacdec ! audio/x-raw,format=S16LE,channels=2,rate=44100 ! filesink name=sink",
        )
        .unwrap()
        .downcast::<gst::Pipeline>()
        .unwrap();
        pipeline
            .by_name("source")
            .unwrap()
            .set_property("location", flac.to_str().unwrap());
        pipeline
            .by_name("sink")
            .unwrap()
            .set_property("location", decoded.to_str().unwrap());
        pipeline.set_state(gst::State::Playing).unwrap();
        let message = pipeline.bus().unwrap().timed_pop_filtered(
            gst::ClockTime::from_seconds(10),
            &[gst::MessageType::Eos, gst::MessageType::Error],
        );
        pipeline.set_state(gst::State::Null).unwrap();
        assert_eq!(message.unwrap().type_(), gst::MessageType::Eos);
        assert_eq!(fs::read(decoded).unwrap(), pcm);
        fs::remove_dir_all(directory).unwrap();
    }
}
