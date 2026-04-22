use chrono::Datelike;
use chrono::Duration;
use gtk::Label;
use gtk4 as gtk;
use gtk4::prelude::WidgetExt;
use hijri_date::HijriDate;

use crate::config::AppConfig;
use crate::i18n::tr;
use crate::location;

fn contains_arabic(text: &str) -> bool {
    text.chars().any(|c| {
        let code = c as u32;
        (0x0600..=0x06FF).contains(&code)
            || (0x0750..=0x077F).contains(&code)
            || (0x08A0..=0x08FF).contains(&code)
            || (0xFB50..=0xFDFF).contains(&code)
            || (0xFE70..=0xFEFF).contains(&code)
    })
}

pub fn refresh_home_ui(
    hijri_label: &Label,
    location_label: &Label,
    lang: &str,
    config: &AppConfig,
) {
    let now = crate::time::effective_now(config);
    let adjusted_now = now + Duration::days(config.hijri_offset);
    let hijri_result = HijriDate::from_gr(
        adjusted_now.year() as usize,
        adjusted_now.month() as usize,
        adjusted_now.day() as usize,
    );

    let hijri_text = match hijri_result {
        Ok(hijri) => {
            let en_months = [
                "Muharram",
                "Safar",
                "Rabi' al-Awwal",
                "Rabi' al-Thani",
                "Jumada al-Ula",
                "Jumada al-Akhirah",
                "Rajab",
                "Sha'ban",
                "Ramadan",
                "Shawwal",
                "Dhu al-Qi'dah",
                "Dhu al-Hijjah",
            ];
            let m_index = hijri.month() - 1;
            let m_name = if m_index < en_months.len() {
                tr(en_months[m_index], lang)
            } else {
                String::from("Unknown")
            };
            format!("{} {} {}", hijri.day(), m_name, hijri.year())
        }
        Err(e) => {
            log::error!("Failed to calculate Hijri date: {e}");
            "—".to_string()
        }
    };
    hijri_label.set_label(&hijri_text);

    let mawaqit_cache = if config.prayer_times_source == crate::config::PrayerTimesSource::Mawaqit {
        config.mawaqit_cache.as_ref()
    } else {
        None
    };

    if let Some(text) =
        location::display_city_label(config.city_name.as_deref(), mawaqit_cache, lang)
    {
        location_label.set_label(&text);
        if contains_arabic(&text) {
            location_label.add_css_class("arabic-text");
        } else {
            location_label.remove_css_class("arabic-text");
        }
    } else {
        location_label.set_label(&format!("{:.2}, {:.2}", config.latitude, config.longitude));
        location_label.remove_css_class("arabic-text");
    }
}
