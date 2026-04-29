use std::cell::RefCell;
use std::rc::Rc;

use chrono::{Datelike, Duration, Local, NaiveDate};
use hijri_date::HijriDate;

use crate::adkar;

use crate::config::AppConfig;
use crate::i18n::tr;
use crate::location;
use crate::notifications::show_notification;
use crate::time::{PrayerEngine, next_prayer_from_schedule, schedule_for_config};

pub struct PrayerState {
    pub hero_text: String,
    pub hijri_text: String,
    pub location_text: String,
    pub next_prayer_name: String,
    pub adhan_playing: bool,
    pub adhan_prayer_name: Option<String>,
}

type IqamahState = Rc<RefCell<Option<(String, chrono::DateTime<chrono::Local>)>>>;

pub fn start_prayer_timer(
    config: Rc<RefCell<AppConfig>>,
    on_state: impl Fn(PrayerState) + 'static,
) {
    use std::sync::atomic::{AtomicBool, Ordering};
    static HAS_CORE_TIMER: AtomicBool = AtomicBool::new(false);
    let is_core_timer = !HAS_CORE_TIMER.swap(true, Ordering::SeqCst);

    let last_notified_prayer: Rc<RefCell<Option<String>>> = Rc::new(RefCell::new(None));
    let last_pre_notified: Rc<RefCell<Option<String>>> = Rc::new(RefCell::new(None));
    let current_adhan_prayer: Rc<RefCell<Option<String>>> = Rc::new(RefCell::new(None));
    let iqamah_state: IqamahState = Rc::new(RefCell::new(None));
    let iqamah_notified: Rc<RefCell<Option<String>>> = Rc::new(RefCell::new(None));

    struct DailyAdkarLists {
        date: NaiveDate,
        morning: Vec<crate::adkar::Dikr>,
        evening: Vec<crate::adkar::Dikr>,
        night: Vec<crate::adkar::Dikr>,
    }

    let default_date = Local::now().naive_local().date() - Duration::days(1);
    let daily_adkar_lists = Rc::new(RefCell::new(DailyAdkarLists {
        date: default_date,
        morning: vec![],
        evening: vec![],
        night: vec![],
    }));

    let last_morning_adkar_1: Rc<RefCell<Option<NaiveDate>>> = Rc::new(RefCell::new(None));
    let last_morning_adkar_2: Rc<RefCell<Option<NaiveDate>>> = Rc::new(RefCell::new(None));
    let last_evening_adkar_1: Rc<RefCell<Option<NaiveDate>>> = Rc::new(RefCell::new(None));
    let last_evening_adkar_2: Rc<RefCell<Option<NaiveDate>>> = Rc::new(RefCell::new(None));
    let last_night_adkar_1: Rc<RefCell<Option<NaiveDate>>> = Rc::new(RefCell::new(None));
    let last_night_adkar_2: Rc<RefCell<Option<NaiveDate>>> = Rc::new(RefCell::new(None));

    let engine_cache: Rc<RefCell<Option<(PrayerEngine, String)>>> = Rc::new(RefCell::new(None));
    let last_mawaqit_attempt_day: Rc<RefCell<Option<String>>> = Rc::new(RefCell::new(None));
    let config_rc = config.clone();

    gtk4::glib::timeout_add_seconds_local(1, move || {
        let config = config_rc.borrow();

        let fingerprint = format!(
            "{}:{}:{:?}:{:?}",
            config.latitude, config.longitude, config.method, config.madhab
        );
        {
            let mut cache = engine_cache.borrow_mut();
            if cache
                .as_ref()
                .map(|(_, f)| f != &fingerprint)
                .unwrap_or(true)
            {
                let engine = PrayerEngine::new(
                    config.latitude,
                    config.longitude,
                    &config.method,
                    &config.madhab,
                );
                *cache = Some((engine, fingerprint));
            }
        }

        let cache = engine_cache.borrow();
        let (_engine, _) = cache.as_ref().unwrap();
        let today = crate::time::effective_today(&config);
        let lang = config.language.clone();

        let now = crate::time::effective_now(&config);
        let adjusted_now = now + Duration::days(config.hijri_offset);
        let hijri_text = match HijriDate::from_gr(
            adjusted_now.year() as usize,
            adjusted_now.month() as usize,
            adjusted_now.day() as usize,
        ) {
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
                let m_name = tr(en_months.get(hijri.month() - 1).unwrap_or(&""), &lang);
                format!("{} {} {}", hijri.day(), m_name, hijri.year())
            }
            Err(e) => {
                log::error!("Failed to calculate Hijri date: {e}");
                "—".to_string()
            }
        };

        let mawaqit_cache =
            if config.prayer_times_source == crate::config::PrayerTimesSource::Mawaqit {
                config.mawaqit_cache.as_ref()
            } else {
                None
            };
        let location_text =
            location::display_city_label(config.city_name.as_deref(), mawaqit_cache, &lang)
                .unwrap_or_else(|| format!("{:.2}, {:.2}", config.latitude, config.longitude));

        if config.prayer_times_source == crate::config::PrayerTimesSource::Mawaqit
            && config.mawaqit_auto_refresh_daily
            && let Some(url) = config.mawaqit_url.clone()
        {
            let today_s = today.to_string();
            let fetched_today = config
                .mawaqit_cache
                .as_ref()
                .map(|c| c.fetched_on.as_str() == today_s.as_str())
                .unwrap_or(false);
            let already_tried_today = last_mawaqit_attempt_day
                .borrow()
                .as_deref()
                .is_some_and(|d| d == today_s.as_str());
            if !fetched_today && !already_tried_today {
                *last_mawaqit_attempt_day.borrow_mut() = Some(today_s.clone());
                let cfg = config_rc.clone();
                gtk4::glib::spawn_future_local(async move {
                    if let Ok(cache) = crate::mawaqit::fetch_mawaqit_cache(&url).await {
                        let mut c = cfg.borrow_mut();
                        c.mawaqit_cache = Some(cache.clone());
                        c.mawaqit_url = Some(cache.url.clone());
                        c.sync_quran_state_from_disk();
                        c.save();
                    }
                });
            }
        }

        let schedule_today = schedule_for_config(&config, today);
        let next = schedule_today
            .as_ref()
            .and_then(|s| next_prayer_from_schedule(s, now))
            .or_else(|| {
                let next_day = today.succ_opt()?;
                let s = schedule_for_config(&config, next_day)?;
                Some(("Fajr".to_string(), s.fajr))
            });

        let adhan_playing = crate::audio::is_playing();
        if !adhan_playing {
            *current_adhan_prayer.borrow_mut() = None;
        }

        if let Some((name, time)) = next {
            let duration = time.signed_duration_since(now);
            let total_seconds = duration.num_seconds();
            let hours = duration.num_hours();
            let minutes = (duration.num_minutes() % 60).abs();
            let seconds = (duration.num_seconds() % 60).abs();

            let hero_text = if total_seconds > 0 {
                format!(
                    "{} {} {:02}:{:02}:{:02}",
                    tr(&name, &lang),
                    tr("in", &lang),
                    hours,
                    minutes,
                    seconds
                )
            } else {
                format!("{} {}", tr("It's time for", &lang), tr(&name, &lang))
            };

            if is_core_timer
                && config.pre_prayer_notify
                && !config.adhan_only_mode
                && total_seconds > 0
                && total_seconds <= (config.pre_prayer_minutes as i64 * 60)
                && name != "Sunrise"
            {
                let mut last_pre = last_pre_notified.borrow_mut();
                if last_pre.as_deref() != Some(name.as_str()) {
                    show_notification(
                        &format!("{} {}", tr("Upcoming Prayer:", &lang), tr(&name, &lang)),
                        &format!(
                            "{} {} {} {}",
                            tr(&name, &lang),
                            tr("is in", &lang),
                            config.pre_prayer_minutes,
                            tr("minutes", &lang)
                        ),
                        false,
                        &tr("Open Khushu", &lang),
                        &tr("Stop Adhan", &lang),
                    );
                    *last_pre = Some(name.clone());
                }
            }

            if is_core_timer && total_seconds <= 0 && total_seconds > -60 {
                let mut last_pray = last_notified_prayer.borrow_mut();
                if last_pray.as_deref() != Some(name.as_str()) {
                    let is_prayer = name != "Sunrise";
                    show_notification(
                        &format!("{} {}", tr("It's time for", &lang), tr(&name, &lang)),
                        &format!("{} {}.", tr("It is now time for", &lang), tr(&name, &lang)),
                        is_prayer,
                        &tr("Open Khushu", &lang),
                        &tr("Stop Adhan", &lang),
                    );

                    if name != "Sunrise" {
                        let path = config
                            .adhan_sound_path
                            .clone()
                            .unwrap_or_else(|| "assets/audio/Madinah.mp3".to_string());
                        if !config.adhan_muted {
                            crate::audio::play_adhan(&path, config.adhan_volume);
                            *current_adhan_prayer.borrow_mut() = Some(name.clone());
                        }
                        let iqamah_mins =
                            config.iqamah_minutes.get(&name).copied().unwrap_or(10) as i64;
                        let iqamah_end = time + chrono::Duration::minutes(iqamah_mins);
                        *iqamah_state.borrow_mut() = Some((name.clone(), iqamah_end));
                        *iqamah_notified.borrow_mut() = None;
                    }

                    *last_pray = Some(name.clone());
                    *last_pre_notified.borrow_mut() = None;
                }
            }

            if is_core_timer && config.adkar_notification_enabled && !config.adhan_only_mode {
                let mut d_lists = daily_adkar_lists.borrow_mut();
                if d_lists.date != today {
                    d_lists.morning = adkar::get_n_random_dikrs("morning", 2);
                    d_lists.evening = adkar::get_n_random_dikrs("evening", 2);
                    d_lists.night = adkar::get_n_random_dikrs("night", 2);
                    d_lists.date = today;
                }

                if let Some(schedule) = schedule_today.as_ref() {
                    let fajr_elapsed = now.signed_duration_since(schedule.fajr).num_seconds();
                    let asr_elapsed = now.signed_duration_since(schedule.asr).num_seconds();
                    let isha_elapsed = now.signed_duration_since(schedule.isha).num_seconds();

                    if (60..120).contains(&fajr_elapsed) {
                        let mut state = last_morning_adkar_1.borrow_mut();
                        if *state != Some(today) {
                            if let Some(dikr) = d_lists.morning.first() {
                                let body = if lang == "ar" {
                                    &dikr.arabic
                                } else {
                                    &dikr.translation
                                };
                                show_notification(
                                    &tr("Morning Adkar", &lang),
                                    body,
                                    false,
                                    &tr("Open Khushu", &lang),
                                    &tr("Stop Adhan", &lang),
                                );
                            }
                            *state = Some(today);
                        }
                    }
                    if (1800..1860).contains(&fajr_elapsed) {
                        let mut state = last_morning_adkar_2.borrow_mut();
                        if *state != Some(today) {
                            if let Some(dikr) = d_lists.morning.get(1) {
                                let body = if lang == "ar" {
                                    &dikr.arabic
                                } else {
                                    &dikr.translation
                                };
                                show_notification(
                                    &tr("Morning Adkar", &lang),
                                    body,
                                    false,
                                    &tr("Open Khushu", &lang),
                                    &tr("Stop Adhan", &lang),
                                );
                            }
                            *state = Some(today);
                        }
                    }

                    if (900..960).contains(&asr_elapsed) {
                        let mut state = last_evening_adkar_1.borrow_mut();
                        if *state != Some(today) {
                            if let Some(dikr) = d_lists.evening.first() {
                                let body = if lang == "ar" {
                                    &dikr.arabic
                                } else {
                                    &dikr.translation
                                };
                                show_notification(
                                    &tr("Evening Adkar", &lang),
                                    body,
                                    false,
                                    &tr("Open Khushu", &lang),
                                    &tr("Stop Adhan", &lang),
                                );
                            }
                            *state = Some(today);
                        }
                    }
                    if (2700..2760).contains(&asr_elapsed) {
                        let mut state = last_evening_adkar_2.borrow_mut();
                        if *state != Some(today) {
                            if let Some(dikr) = d_lists.evening.get(1) {
                                let body = if lang == "ar" {
                                    &dikr.arabic
                                } else {
                                    &dikr.translation
                                };
                                show_notification(
                                    &tr("Evening Adkar", &lang),
                                    body,
                                    false,
                                    &tr("Open Khushu", &lang),
                                    &tr("Stop Adhan", &lang),
                                );
                            }
                            *state = Some(today);
                        }
                    }

                    if (1800..1860).contains(&isha_elapsed) {
                        let mut state = last_night_adkar_1.borrow_mut();
                        if *state != Some(today) {
                            if let Some(dikr) = d_lists.night.first() {
                                let body = if lang == "ar" {
                                    &dikr.arabic
                                } else {
                                    &dikr.translation
                                };
                                show_notification(
                                    &tr("Night Adkar", &lang),
                                    body,
                                    false,
                                    &tr("Open Khushu", &lang),
                                    &tr("Stop Adhan", &lang),
                                );
                            }
                            *state = Some(today);
                        }
                    }
                    if (3600..3660).contains(&isha_elapsed) {
                        let mut state = last_night_adkar_2.borrow_mut();
                        if *state != Some(today) {
                            if let Some(dikr) = d_lists.night.get(1) {
                                let body = if lang == "ar" {
                                    &dikr.arabic
                                } else {
                                    &dikr.translation
                                };
                                show_notification(
                                    &tr("Night Adkar", &lang),
                                    body,
                                    false,
                                    &tr("Open Khushu", &lang),
                                    &tr("Stop Adhan", &lang),
                                );
                            }
                            *state = Some(today);
                        }
                    }
                }
            }

            if total_seconds < -1000 {
                *last_notified_prayer.borrow_mut() = None;
                *iqamah_state.borrow_mut() = None;
            }

            let iqamah_hero = {
                let state = iqamah_state.borrow();
                state.as_ref().and_then(|(iq_name, iq_end)| {
                    let remaining = iq_end.signed_duration_since(now).num_seconds();
                    if remaining > 0 {
                        let m = remaining / 60;
                        let s = remaining % 60;
                        Some(format!(
                            "{} {} {:02}:{:02}",
                            tr("Iqamah", &lang),
                            tr(iq_name, &lang),
                            m,
                            s
                        ))
                    } else {
                        None
                    }
                })
            };

            if is_core_timer {
                let should_notify = {
                    let state = iqamah_state.borrow();
                    let notified = iqamah_notified.borrow();
                    state.as_ref().is_some_and(|(iq_name, iq_end)| {
                        let remaining = iq_end.signed_duration_since(now).num_seconds();
                        remaining <= 0 && remaining > -60 && notified.as_deref() != Some(iq_name)
                    })
                };
                if should_notify
                    && config.iqamah_notify
                    && !config.adhan_only_mode
                    && let Some((iq_name, _)) = iqamah_state.borrow().as_ref()
                {
                    show_notification(
                        &format!("{} {}", tr("Iqamah", &lang), tr(iq_name, &lang)),
                        &format!(
                            "{} {}.",
                            tr("It is time for Iqamah of", &lang),
                            tr(iq_name, &lang)
                        ),
                        false,
                        &tr("Open Khushu", &lang),
                        &tr("Stop Adhan", &lang),
                    );
                    *iqamah_notified.borrow_mut() = Some(iq_name.clone());
                }
            }

            let final_hero = iqamah_hero.unwrap_or(hero_text);

            on_state(PrayerState {
                hero_text: final_hero,
                hijri_text,
                location_text,
                next_prayer_name: name,
                adhan_playing,
                adhan_prayer_name: current_adhan_prayer.borrow().clone(),
            });
        }

        gtk4::glib::ControlFlow::Continue
    });
}
