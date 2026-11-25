use gst::prelude::*;
use gstreamer as gst;
use std::path::Path;
use std::sync::{Arc, Mutex};

#[derive(Debug)]
struct QueuedTrack {
    path: String,
    uri: String,
}

#[derive(Debug, Default)]
struct Queue {
    next: Option<QueuedTrack>,
    armed: Option<QueuedTrack>,
}

#[derive(Debug, Default)]
pub(super) struct Gapless(Arc<Mutex<Queue>>);

impl Gapless {
    pub(super) fn connect(&self, playbin: &gst::Element) {
        let queue = Arc::clone(&self.0);
        playbin.connect("about-to-finish", false, move |values| {
            let Some(playbin) = values[0].get::<gst::Element>().ok() else {
                return None;
            };
            let uri = queue.lock().ok().and_then(|mut queue| {
                if queue.armed.is_some() {
                    return None;
                }
                let next = queue.next.take()?;
                let uri = next.uri.clone();
                queue.armed = Some(next);
                Some(uri)
            });
            if let Some(uri) = uri {
                playbin.set_property("uri", uri);
            }
            None
        });
    }

    pub(super) fn stage(&self, current: &str, path: Option<&str>) {
        let next = path.filter(|path| *path != current).and_then(|path| {
            let file = Path::new(path);
            if !file.is_file() {
                return None;
            }
            Some(QueuedTrack {
                path: path.into(),
                uri: gst::glib::filename_to_uri(file, None).ok()?.to_string(),
            })
        });
        if let Ok(mut queue) = self.0.lock() {
            queue.next = next.filter(|track| {
                queue
                    .armed
                    .as_ref()
                    .is_none_or(|armed| armed.uri != track.uri)
            });
        }
    }

    pub(super) fn reset(&self) {
        if let Ok(mut queue) = self.0.lock() {
            *queue = Queue::default();
        }
    }

    #[cfg(any(target_os = "windows", test))]
    pub(super) fn armed_path(&self) -> Option<String> {
        self.0
            .lock()
            .ok()?
            .armed
            .as_ref()
            .map(|track| track.path.clone())
    }

    pub(super) fn take_current(&self, playbin: &gst::Element) -> Option<String> {
        let current_uri = playbin.property::<Option<String>>("current-uri")?;
        let mut queue = self.0.lock().ok()?;
        if queue.armed.as_ref()?.uri != current_uri {
            return None;
        }
        queue.next = None;
        queue.armed.take().map(|track| track.path)
    }
}
