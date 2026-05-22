use rodio::{Decoder, OutputStreamBuilder, Sink};
use std::collections::HashMap;
use std::io::Cursor;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::thread;

use adw::prelude::*;
use libadwaita as adw;

use crate::config::AppConfig;

static AUDIO_SENDER: OnceLock<Sender<AudioCommand>> = OnceLock::new();
static IS_PLAYING: AtomicBool = AtomicBool::new(false);
static BUILTIN_AUDIO: OnceLock<HashMap<String, Vec<u8>>> = OnceLock::new();

enum AudioCommand {
    Play(String, f32),
    Stop,
}

fn ensure_audio_thread() -> &'static Sender<AudioCommand> {
    AUDIO_SENDER.get_or_init(|| {
        let (tx, rx) = channel();
        thread::spawn(move || {
            run_audio_loop(rx);
        });
        tx
    })
}

pub fn preload_builtin_audio() {
    BUILTIN_AUDIO.get_or_init(|| {
        let mut map = HashMap::new();
        let presets = [
            ("Madinah.mp3", "audio/Madinah.mp3"),
            ("Makkah.mp3", "audio/Makkah.mp3"),
        ];
        for (key, resource_path) in presets {
            if let Ok(bytes) = gtk4::gio::resources_lookup_data(
                &format!("/io/github/sniper1720/khushu/{resource_path}"),
                gtk4::gio::ResourceLookupFlags::NONE,
            ) {
                map.insert(key.to_string(), bytes.to_vec());
            } else {
                log::error!("Failed to preload builtin audio: {resource_path}");
            }
        }
        map
    });
}

pub fn play_adhan(path_str: &str, volume: f32) {
    let _ = ensure_audio_thread().send(AudioCommand::Play(path_str.to_string(), volume));
}

pub fn stop() {
    let _ = ensure_audio_thread().send(AudioCommand::Stop);
}

pub fn is_playing() -> bool {
    IS_PLAYING.load(Ordering::Relaxed)
}

fn validate_audio_file(path: &str) -> bool {
    std::fs::File::open(path)
        .ok()
        .and_then(|file| Decoder::new(std::io::BufReader::new(file)).ok())
        .is_some()
}

struct ValidationResult {
    path: String,
    valid: bool,
    lang: String,
}

static VALIDATION_CHANNEL: OnceLock<Sender<ValidationResult>> = OnceLock::new();

fn ensure_validation_thread() -> &'static Sender<ValidationResult> {
    VALIDATION_CHANNEL.get_or_init(|| {
        let (tx, rx): (Sender<ValidationResult>, _) = channel();
        thread::spawn(move || {
            while let Ok(result) = rx.recv() {
                let path = result.path;
                let valid = result.valid;
                let _lang = result.lang;
                gtk4::glib::spawn_future_local(async move {
                    if valid {
                        let c = AppConfig::load();
                        c.set_adhan_sound_path(Some(path.clone()));
                        c.save();
                    }
                });
            }
        });
        tx
    })
}

pub fn validate_audio_async(
    path: String,
    combo: adw::ComboRow,
    lang: String,
    parent: adw::ApplicationWindow,
) {
    let valid = validate_audio_file(&path);
    let path_for_save = path.clone();
    if valid {
        let _ = ensure_validation_thread().send(ValidationResult {
            path: path_for_save,
            valid: true,
            lang,
        });
        gtk4::glib::spawn_future_local(async move {
            combo.set_subtitle(&path);
        });
    } else if let Some(overlay) = find_toast_overlay(&parent) {
        gtk4::glib::spawn_future_local(async move {
            overlay.add_toast(adw::Toast::new(&crate::i18n::tr(
                "File not usable or unsupported format",
                &lang,
            )));
        });
    }
}

fn find_toast_overlay(window: &adw::ApplicationWindow) -> Option<adw::ToastOverlay> {
    let mut child = window.first_child();
    while let Some(w) = child {
        if let Some(o) = w.downcast_ref::<adw::ToastOverlay>() {
            return Some(o.clone());
        }
        child = w.next_sibling();
    }
    None
}

fn get_builtin_bytes(path_str: &str) -> Option<&'static [u8]> {
    let file_name = path_str
        .trim_start_matches("assets/audio/")
        .trim_start_matches("assets/");
    BUILTIN_AUDIO
        .get()
        .and_then(|map| map.get(file_name))
        .map(|v| v.as_slice())
}

const FALLBACK_KEY: &str = "Madinah.mp3";

fn try_play_custom(path_str: &str, sink: &Sink) -> bool {
    if let Ok(file) = std::fs::File::open(path_str) {
        if let Ok(decoder) = Decoder::new(std::io::BufReader::new(file)) {
            sink.append(decoder);
            return true;
        } else {
            log::error!("Failed to decode audio file: {}", path_str);
        }
    } else {
        log::error!("Failed to open audio file: {}", path_str);
    }

    false
}

fn try_play_builtin(path_str: &str, sink: &Sink) -> bool {
    if let Some(bytes) = get_builtin_bytes(path_str)
        && let Ok(decoder) = Decoder::new(Cursor::new(bytes))
    {
        sink.append(decoder);
        return true;
    }

    log::error!("Builtin audio not available: {}", path_str);
    false
}

fn run_audio_loop(rx: Receiver<AudioCommand>) {
    let stream = match OutputStreamBuilder::open_default_stream() {
        Ok(s) => s,
        Err(e) => {
            log::error!("Failed to open default audio output stream: {}", e);
            return;
        }
    };

    let mut current_sink: Option<Sink> = None;

    loop {
        match rx.recv_timeout(std::time::Duration::from_millis(200)) {
            Ok(command) => match command {
                AudioCommand::Play(path_str, volume) => {
                    current_sink = None;
                    IS_PLAYING.store(true, Ordering::Relaxed);
                    crate::settings_ui::on_audio_state_changed(true);

                    let sink = Sink::connect_new(stream.mixer());
                    sink.set_volume(volume.clamp(0.0, 1.0));

                    let is_asset = path_str.starts_with("assets/");

                    let played = if is_asset {
                        try_play_builtin(&path_str, &sink)
                    } else {
                        try_play_custom(&path_str, &sink)
                    };

                    if !played {
                        log::warn!(
                            "Audio playback failed for '{}', falling back to builtin",
                            path_str
                        );
                        if !try_play_builtin(FALLBACK_KEY, &sink) {
                            log::error!("No fallback audio available");
                            IS_PLAYING.store(false, Ordering::Relaxed);
                            crate::settings_ui::on_audio_state_changed(false);
                            continue;
                        }
                    }

                    current_sink = Some(sink);
                }
                AudioCommand::Stop => {
                    current_sink = None;
                    IS_PLAYING.store(false, Ordering::Relaxed);
                    crate::settings_ui::on_audio_state_changed(false);
                }
            },
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                if let Some(sink) = current_sink.as_ref()
                    && sink.empty()
                {
                    IS_PLAYING.store(false, Ordering::Relaxed);
                    crate::settings_ui::on_audio_state_changed(false);
                    current_sink = None;
                }
            }
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
        }
    }
}
