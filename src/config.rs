use crate::security;
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::collections::HashMap;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::rc::Rc;

use gtk4::glib;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum LocationMode {
    Manual,
    City,
    Auto,
}

#[derive(Serialize, Deserialize, Default, Debug, Clone, PartialEq)]
pub enum PrayerTimesSource {
    #[default]
    Calculated,
    Mawaqit,
}

#[derive(Serialize, Deserialize, Default, Debug, Clone, PartialEq)]
#[allow(clippy::upper_case_acronyms)]
pub enum CalculationMethod {
    #[default]
    MWL,
    ISNA,
    Egypt,
    Makkah,
    Karachi,
    Dubai,
    MoonsightingCommittee,
    Kuwait,
    Qatar,
    Singapore,
    Turkey,
}

#[derive(Serialize, Deserialize, Default, Debug, Clone, PartialEq)]
pub enum MadhabChoice {
    #[default]
    Shafi,
    Hanafi,
}

#[derive(Serialize, Deserialize, Default, Debug, Clone, PartialEq)]
pub enum ThemeMode {
    #[serde(rename = "system")]
    #[default]
    System,
    #[serde(rename = "light")]
    Light,
    #[serde(rename = "dark")]
    Dark,
}

#[derive(Serialize, Deserialize, Default, Debug, Clone, PartialEq)]
pub enum TimezoneMode {
    #[serde(rename = "auto")]
    #[default]
    Auto,
    #[serde(rename = "named")]
    Named(String),
    #[serde(rename = "utc_offset")]
    UtcOffset(i32),
}

fn default_volume() -> f32 {
    1.0
}

fn default_autostart() -> bool {
    true
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct QuranBookmark {
    pub page: u32,
    #[serde(default)]
    pub surah: u32,
    #[serde(default)]
    pub verse: u32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MawaqitCache {
    pub url: String,
    #[serde(default)]
    pub mosque_name: Option<String>,
    #[serde(default)]
    pub timezone: Option<String>,
    #[serde(default)]
    pub latitude: Option<f64>,
    #[serde(default)]
    pub longitude: Option<f64>,
    #[serde(default)]
    pub country_code: Option<String>,
    pub year: i32,
    #[serde(default)]
    pub months: Vec<std::collections::BTreeMap<u32, [String; 6]>>,
    #[serde(default)]
    pub fetched_on: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AppConfig {
    #[serde(skip)]
    pub latitude: f64,
    #[serde(skip)]
    pub longitude: f64,
    pub method: CalculationMethod,
    pub madhab: MadhabChoice,
    pub location_mode: LocationMode,
    pub city_name: Option<String>,
    pub adhan_sound_path: Option<String>,
    pub pre_prayer_notify: bool,
    pub pre_prayer_minutes: u32,
    pub hijri_offset: i64,
    #[serde(default)]
    pub favorites: Vec<String>,
    #[serde(default)]
    pub adkar_notification_enabled: bool,
    #[serde(default)]
    pub language: String,
    #[serde(default)]
    pub theme: ThemeMode,
    #[serde(default)]
    pub is_configured: bool,
    #[serde(default = "default_volume")]
    pub adhan_volume: f32,
    #[serde(default)]
    pub adhan_muted: bool,
    #[serde(default = "default_autostart")]
    pub autostart: bool,

    #[serde(default)]
    pub enc_lat: String,
    #[serde(default)]
    pub enc_lon: String,

    #[serde(default)]
    pub quran_bookmark_surah: Option<u32>,
    #[serde(default)]
    pub quran_bookmark_page: Option<u32>,

    #[serde(default)]
    pub quran_bookmarks: Vec<QuranBookmark>,

    #[serde(default)]
    pub quran_last_surah: Option<u32>,
    #[serde(default)]
    pub quran_last_page: Option<u32>,

    #[serde(default)]
    pub prayer_times_source: PrayerTimesSource,

    #[serde(default)]
    pub mawaqit_url: Option<String>,
    #[serde(default)]
    pub mawaqit_auto_refresh_daily: bool,
    #[serde(default)]
    pub mawaqit_cache: Option<MawaqitCache>,

    #[serde(default)]
    pub enc_mawaqit_url: String,
    #[serde(default)]
    pub enc_mawaqit_cache: String,

    #[serde(default)]
    pub timezone_override_minutes: Option<i32>,

    #[serde(default)]
    pub timezone_mode: TimezoneMode,

    #[serde(default = "default_quran_arabic_font_px")]
    pub quran_arabic_font_px: f64,
    #[serde(default = "default_quran_translation_font_px")]
    pub quran_translation_font_px: f64,
    #[serde(default = "default_quran_line_height")]
    pub quran_line_height: f64,

    #[serde(default = "default_global_arabic_font_family")]
    pub global_arabic_font_family: String,
    #[serde(default = "default_global_ui_font_family")]
    pub global_ui_font_family: String,

    #[serde(default = "default_iqamah_minutes")]
    pub iqamah_minutes: HashMap<String, u32>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            latitude: 36.75,
            longitude: 3.05,
            method: CalculationMethod::MWL,
            madhab: MadhabChoice::Shafi,
            location_mode: LocationMode::Manual,
            city_name: None,
            adhan_sound_path: None,
            pre_prayer_notify: true,
            pre_prayer_minutes: 15,
            hijri_offset: 0,
            favorites: Vec::new(),
            adkar_notification_enabled: true,
            language: "auto".to_string(),
            theme: ThemeMode::System,
            is_configured: false,
            adhan_volume: 1.0,
            adhan_muted: false,
            autostart: true,
            enc_lat: String::new(),
            enc_lon: String::new(),
            quran_bookmark_surah: None,
            quran_bookmark_page: None,
            quran_bookmarks: Vec::new(),
            quran_last_surah: None,
            quran_last_page: None,
            prayer_times_source: PrayerTimesSource::Calculated,
            mawaqit_url: None,
            mawaqit_auto_refresh_daily: false,
            mawaqit_cache: None,
            enc_mawaqit_url: String::new(),
            enc_mawaqit_cache: String::new(),
            timezone_override_minutes: None,
            timezone_mode: TimezoneMode::Auto,
            quran_arabic_font_px: default_quran_arabic_font_px(),
            quran_translation_font_px: default_quran_translation_font_px(),
            quran_line_height: default_quran_line_height(),
            global_arabic_font_family: default_global_arabic_font_family(),
            global_ui_font_family: default_global_ui_font_family(),
            iqamah_minutes: default_iqamah_minutes(),
        }
    }
}

fn default_quran_arabic_font_px() -> f64 {
    22.0
}

fn default_quran_translation_font_px() -> f64 {
    14.0
}

fn default_quran_line_height() -> f64 {
    1.0
}

fn default_global_arabic_font_family() -> String {
    "Amiri, Noto Sans Arabic".to_string()
}

fn default_global_ui_font_family() -> String {
    "Cantarell, sans-serif".to_string()
}

fn default_iqamah_minutes() -> HashMap<String, u32> {
    let mut m = HashMap::new();
    m.insert("Fajr".to_string(), 20);
    m.insert("Dhuhr".to_string(), 10);
    m.insert("Asr".to_string(), 10);
    m.insert("Maghrib".to_string(), 5);
    m.insert("Isha".to_string(), 10);
    m
}

impl AppConfig {
    pub fn sync_quran_state_from_disk(&mut self) {
        let latest = Self::load();
        self.quran_arabic_font_px = latest.quran_arabic_font_px;
        self.quran_translation_font_px = latest.quran_translation_font_px;
        self.quran_line_height = latest.quran_line_height;
        self.quran_bookmark_surah = latest.quran_bookmark_surah;
        self.quran_bookmark_page = latest.quran_bookmark_page;
        self.quran_bookmarks = latest.quran_bookmarks;
        self.quran_last_surah = latest.quran_last_surah;
        self.quran_last_page = latest.quran_last_page;
    }

    pub fn load() -> Self {
        let path = Self::config_path();
        if path.exists()
            && let Ok(content) = fs::read_to_string(&path)
            && let Ok(mut config) = serde_json::from_str::<Self>(&content)
        {
            log::info!("Configuration loaded from {:?}", path);
            if !config.enc_lat.is_empty()
                && let Ok(dec) = security::deobfuscate(&config.enc_lat)
            {
                config.latitude = dec.parse().unwrap_or(36.75);
            }
            if !config.enc_lon.is_empty()
                && let Ok(dec) = security::deobfuscate(&config.enc_lon)
            {
                config.longitude = dec.parse().unwrap_or(3.05);
            }
            if config.quran_bookmarks.is_empty()
                && let (Some(surah), Some(page)) =
                    (config.quran_bookmark_surah, config.quran_bookmark_page)
            {
                config.quran_bookmarks.push(QuranBookmark {
                    page,
                    surah,
                    verse: 1,
                });
            }

            if config.mawaqit_url.is_none()
                && !config.enc_mawaqit_url.is_empty()
                && let Ok(dec) = security::deobfuscate(&config.enc_mawaqit_url)
            {
                config.mawaqit_url = Some(dec);
            }

            if config.mawaqit_cache.is_none()
                && !config.enc_mawaqit_cache.is_empty()
                && let Ok(dec) = security::deobfuscate(&config.enc_mawaqit_cache)
                && let Ok(cache) = serde_json::from_str::<MawaqitCache>(&dec)
            {
                config.mawaqit_cache = Some(cache);
            }
            return config;
        }
        log::info!("No existing configuration found, using defaults");
        Self::default()
    }

    pub fn save(&self) {
        let path = Self::config_path();
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }

        let mut encrypted_self = self.clone();

        if let Ok(enc) = security::obfuscate(&self.latitude.to_string()) {
            encrypted_self.enc_lat = enc;
        }

        if let Ok(enc) = security::obfuscate(&self.longitude.to_string()) {
            encrypted_self.enc_lon = enc;
        }

        if let Some(url) = &self.mawaqit_url
            && let Ok(enc) = security::obfuscate(url)
        {
            encrypted_self.enc_mawaqit_url = enc;
            encrypted_self.mawaqit_url = None;
        }

        if let Some(cache) = &self.mawaqit_cache
            && let Ok(json) = serde_json::to_string(cache)
            && let Ok(enc) = security::obfuscate(&json)
        {
            encrypted_self.enc_mawaqit_cache = enc;
            encrypted_self.mawaqit_cache = None;
        }

        if let Ok(content) = serde_json::to_string_pretty(&encrypted_self)
            && fs::write(&path, &content).is_ok()
        {
            let _ = fs::set_permissions(&path, fs::Permissions::from_mode(0o600));
            log::info!("Configuration obfuscated and saved to {:?}", path);
        } else {
            log::error!("Failed to save configuration to {:?}", path);
        }
    }

    pub fn save_shared(config: &Rc<RefCell<AppConfig>>) {
        let mut cfg = config.borrow_mut();
        cfg.sync_quran_state_from_disk();
        cfg.save();
    }

    pub fn config_path() -> PathBuf {
        let mut path = glib::user_config_dir();
        path.push("khushu");
        path.push("config.json");
        path
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config_arabic_font_family() {
        let config = AppConfig::default();
        assert_eq!(config.global_arabic_font_family, "Amiri, Noto Sans Arabic");
    }

    #[test]
    fn test_default_config_ui_font_family() {
        let config = AppConfig::default();
        assert_eq!(config.global_ui_font_family, "Cantarell, sans-serif");
    }

    #[test]
    fn test_default_config_quran_font_sizes() {
        let config = AppConfig::default();
        assert_eq!(config.quran_arabic_font_px, 22.0);
        assert_eq!(config.quran_translation_font_px, 14.0);
        assert_eq!(config.quran_line_height, 1.0);
    }

    #[test]
    fn test_default_config_location() {
        let config = AppConfig::default();
        assert_eq!(config.latitude, 36.75);
        assert_eq!(config.longitude, 3.05);
        assert_eq!(config.location_mode, LocationMode::Manual);
    }

    #[test]
    fn test_default_config_iqamah_minutes() {
        let config = AppConfig::default();
        let iqamah = &config.iqamah_minutes;
        assert_eq!(iqamah.get("Fajr"), Some(&20));
        assert_eq!(iqamah.get("Dhuhr"), Some(&10));
        assert_eq!(iqamah.get("Asr"), Some(&10));
        assert_eq!(iqamah.get("Maghrib"), Some(&5));
        assert_eq!(iqamah.get("Isha"), Some(&10));
    }
}
