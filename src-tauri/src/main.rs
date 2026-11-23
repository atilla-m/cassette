// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    #[cfg(all(windows, not(debug_assertions)))]
    if let Err(problem) = prepare_bundled_gstreamer() {
        show_windows_startup_error(&problem);
        std::process::exit(1);
    }
    cassette_lib::run()
}

#[cfg(all(windows, not(debug_assertions)))]
fn prepare_bundled_gstreamer() -> Result<(), String> {
    use std::{env, ffi::OsString, fs};

    let executable = env::current_exe().map_err(|error| error.to_string())?;
    let directory = executable
        .parent()
        .ok_or_else(|| "Could not locate the Cassette installation directory.".to_owned())?;
    let core = directory.join("gstreamer-1.0-0.dll");
    let plugins = directory.join("lib").join("gstreamer-1.0");
    let scanner = directory
        .join("libexec")
        .join("gstreamer-1.0")
        .join("gst-plugin-scanner.exe");
    let manifest = directory
        .join("third-party")
        .join("gstreamer")
        .join("manifest.json");
    if !core.is_file() || !plugins.is_dir() || !scanner.is_file() || !manifest.is_file() {
        return Err("This installation is missing its bundled GStreamer runtime. Reinstall Cassette from the verified installer; no separate GStreamer installation or PATH edit is needed.".to_owned());
    }

    // The app-local DLLs are found by the Windows loader beside cassette.exe.
    // The scanner is a child process, so it also needs that directory on its
    // private PATH; this does not change the user's or system's PATH.
    let mut path = OsString::from(directory.as_os_str());
    if let Some(existing) = env::var_os("PATH") {
        path.push(";");
        path.push(existing);
    }
    env::set_var("PATH", path);
    env::set_var("GST_PLUGIN_PATH_1_0", &plugins);
    env::set_var("GST_PLUGIN_SYSTEM_PATH_1_0", "");
    env::set_var("GST_PLUGIN_SCANNER_1_0", &scanner);
    if let Some(local_data) = env::var_os("LOCALAPPDATA") {
        let cache = std::path::PathBuf::from(local_data).join("Cassette");
        if fs::create_dir_all(&cache).is_ok() {
            env::set_var("GST_REGISTRY_1_0", cache.join("gstreamer-registry.bin"));
        }
    }
    Ok(())
}

#[cfg(all(windows, not(debug_assertions)))]
fn show_windows_startup_error(problem: &str) {
    use std::{ffi::c_void, os::windows::ffi::OsStrExt};

    #[link(name = "user32")]
    extern "system" {
        fn MessageBoxW(
            window: *mut c_void,
            text: *const u16,
            caption: *const u16,
            flags: u32,
        ) -> i32;
    }
    let message: Vec<u16> = std::ffi::OsStr::new(problem)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    let title: Vec<u16> = std::ffi::OsStr::new("Cassette cannot start")
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    unsafe { MessageBoxW(std::ptr::null_mut(), message.as_ptr(), title.as_ptr(), 0x10) };
}
