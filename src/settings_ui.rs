use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

use adw::prelude::*;
use adw::{ComboRow, PreferencesGroup};
use gtk::{Button, ListBox, StringList};
use gtk4 as gtk;
use libadwaita as adw;

use crate::config::{
    AppConfig, CalculationMethod, LocationMode, MadhabChoice, PrayerTimesSource, TimezoneMode,
};
use crate::i18n::tr;
use crate::location;
use crate::notifications;

fn set_audio_toggle_button_label(btn: &Button, lang: &str, idle_label_key: &str, is_playing: bool) {
    let label = if is_playing {
        tr("⏹ Stop Adhan", lang)
    } else {
        tr(idle_label_key, lang)
    };
    btn.set_label(&label);
}

fn bind_audio_toggle_button_sync(
    btn: &Button,
    current_lang: Rc<RefCell<String>>,
    idle_label_key: &'static str,
) {
    let btn_weak = btn.downgrade();
    gtk::glib::timeout_add_local(std::time::Duration::from_millis(200), move || {
        if let Some(btn) = btn_weak.upgrade() {
            let lang = current_lang.borrow().clone();
            set_audio_toggle_button_label(&btn, &lang, idle_label_key, crate::audio::is_playing());
            gtk::glib::ControlFlow::Continue
        } else {
            gtk::glib::ControlFlow::Break
        }
    });
}

fn finish_entry_row_interaction(row: &adw::EntryRow) {
    if let Some(root) = row.root() {
        root.set_focus(Option::<&gtk::Widget>::None);
    }
}

fn append_settings_section_heading(
    settings_box: &gtk::Box,
    title: &str,
    description: Option<&str>,
    margin_top: i32,
) {
    let heading = gtk::Label::builder()
        .label(title)
        .css_classes(["title-4"])
        .halign(gtk::Align::Start)
        .xalign(0.0)
        .margin_top(margin_top)
        .margin_bottom(if description.is_some() { 4 } else { 12 })
        .build();
    settings_box.append(&heading);

    if let Some(desc) = description {
        let desc_label = gtk::Label::builder()
            .label(desc)
            .css_classes(["dim-label"])
            .hexpand(true)
            .halign(gtk::Align::Fill)
            .xalign(0.0)
            .wrap(true)
            .wrap_mode(gtk::pango::WrapMode::Word)
            .margin_bottom(12)
            .build();
        settings_box.append(&desc_label);
    }
}

pub struct SettingsUiParams<'a> {
    pub settings_box: &'a gtk::Box,
    pub config: std::rc::Rc<std::cell::RefCell<crate::config::AppConfig>>,
    pub list_box_rc: std::rc::Rc<gtk::ListBox>,
    pub window: &'a adw::ApplicationWindow,
    pub current_lang: std::rc::Rc<std::cell::RefCell<String>>,
    pub loc_tx: std::sync::mpsc::Sender<(f64, f64, Option<String>)>,
    pub refresh_calendar: std::rc::Rc<dyn Fn()>,
    pub lang_row: Option<&'a adw::ComboRow>,
}

pub fn setup_settings_ui<'a>(params: SettingsUiParams<'a>) {
    let SettingsUiParams {
        settings_box,
        config,
        list_box_rc,
        window,
        current_lang,
        loc_tx,
        refresh_calendar,
        lang_row,
    } = params;
    let lang_val = current_lang.borrow().clone();

    while let Some(child) = settings_box.first_child() {
        settings_box.remove(&child);
    }

    append_settings_section_heading(
        settings_box,
        &tr("General", &lang_val),
        Some(&tr(
            "Customize the app's appearance and startup behavior.",
            &lang_val,
        )),
        0,
    );

    let general_group = PreferencesGroup::new();
    general_group.set_margin_bottom(24);
    settings_box.append(&general_group);

    if let Some(row) = lang_row {
        general_group.add(row);
    }

    let theme_model = StringList::new(&[
        &tr("System Default", &lang_val),
        &tr("Light", &lang_val),
        &tr("Dark", &lang_val),
    ]);
    let theme_row = ComboRow::builder()
        .title(tr("Theme", &lang_val))
        .model(&theme_model)
        .build();

    match config.borrow().theme {
        crate::config::ThemeMode::Light => theme_row.set_selected(1),
        crate::config::ThemeMode::Dark => theme_row.set_selected(2),
        _ => theme_row.set_selected(0),
    }

    let config_theme = config.clone();
    theme_row.connect_selected_notify(move |row| {
        let manager = adw::StyleManager::default();
        let new_theme = match row.selected() {
            1 => crate::config::ThemeMode::Light,
            2 => crate::config::ThemeMode::Dark,
            _ => crate::config::ThemeMode::System,
        };

        match new_theme {
            crate::config::ThemeMode::Light => {
                manager.set_color_scheme(adw::ColorScheme::ForceLight)
            }
            crate::config::ThemeMode::Dark => {
                manager.set_color_scheme(adw::ColorScheme::PreferDark)
            }
            crate::config::ThemeMode::System => manager.set_color_scheme(adw::ColorScheme::Default),
        }

        config_theme.borrow_mut().theme = new_theme;
        AppConfig::save_shared(&config_theme);
    });
    general_group.add(&theme_row);

    let autostart_toggle = adw::SwitchRow::builder()
        .title(tr("Start Automatically", &lang_val))
        .subtitle(tr(
            "Run Khushu in the background when you log in.",
            &lang_val,
        ))
        .build();
    autostart_toggle.set_active(config.borrow().autostart);

    let config_autostart = config.clone();
    autostart_toggle.connect_active_notify(move |row| {
        let is_active = row.is_active();
        config_autostart.borrow_mut().autostart = is_active;
        AppConfig::save_shared(&config_autostart);
        crate::autostart::sync(is_active);
    });
    general_group.add(&autostart_toggle);

    append_settings_section_heading(
        settings_box,
        &tr("Prayer Setup", &lang_val),
        Some(&tr(
            "Set your location, prayer times source, timezone, calculation methods, and Iqamah delays for each prayer.",
            &lang_val,
        )),
        24,
    );

    let location_group = PreferencesGroup::builder()
        .title(gtk::glib::markup_escape_text(&tr(
            "Location & Source",
            &lang_val,
        )))
        .description(tr(
            "Set your location and choose the prayer times data source.",
            &lang_val,
        ))
        .build();
    location_group.set_margin_top(0);
    location_group.set_margin_bottom(24);
    settings_box.append(&location_group);

    let modes_strings = [
        tr("Manual Coordinates", &lang_val),
        tr("City Selection", &lang_val),
        tr("Auto (GPS/Network)", &lang_val),
    ];
    let modes_slices: Vec<&str> = modes_strings.iter().map(|s| s.as_str()).collect();
    let modes = StringList::new(&modes_slices);
    let mode_row = ComboRow::builder()
        .title(tr("Location Method", &lang_val))
        .model(&modes)
        .build();

    let current_mode = config.borrow().location_mode.clone();
    mode_row.set_selected(match current_mode {
        LocationMode::Manual => 0,
        LocationMode::City => 1,
        LocationMode::Auto => 2,
    });

    let lat_row = adw::SpinRow::builder()
        .title(tr("Latitude", &lang_val))
        .adjustment(&gtk::Adjustment::new(
            config.borrow().latitude,
            -90.0,
            90.0,
            0.01,
            0.0,
            0.0,
        ))
        .digits(4)
        .build();

    let config_lat = config.clone();
    let list_box_lat = list_box_rc.clone();
    lat_row.adjustment().connect_value_changed(move |adj| {
        config_lat.borrow_mut().latitude = adj.value();
        AppConfig::save_shared(&config_lat);
        refresh_prayers(&config_lat.borrow(), &list_box_lat);
    });

    let lon_row = adw::SpinRow::builder()
        .title(tr("Longitude", &lang_val))
        .adjustment(&gtk::Adjustment::new(
            config.borrow().longitude,
            -180.0,
            180.0,
            0.01,
            0.0,
            0.0,
        ))
        .digits(4)
        .build();

    let config_lon = config.clone();
    let list_box_lon = list_box_rc.clone();
    lon_row.adjustment().connect_value_changed(move |adj| {
        config_lon.borrow_mut().longitude = adj.value();
        AppConfig::save_shared(&config_lon);
        refresh_prayers(&config_lon.borrow(), &list_box_lon);
    });

    let status_row = adw::ActionRow::builder()
        .title(tr("Location Status", &lang_val))
        .visible(false)
        .build();
    status_row.add_css_class("error");
    let status_row_clone = status_row.clone();
    let status_row_clone2 = status_row.clone();

    let city_row = adw::EntryRow::builder()
        .title(tr("City Search", &lang_val))
        .build();

    if config.borrow().location_mode == LocationMode::City {
        let (city_name, mawaqit_cache) = {
            let cfg = config.borrow();
            let mawaqit_cache = if cfg.prayer_times_source == PrayerTimesSource::Mawaqit {
                cfg.mawaqit_cache.clone()
            } else {
                None
            };
            (cfg.city_name.clone(), mawaqit_cache)
        };
        if let Some(text) =
            location::display_city_label(city_name.as_deref(), mawaqit_cache.as_ref(), &lang_val)
        {
            city_row.set_text(&text);
        }
    }

    let city_btn = Button::with_label(&tr("Search", &lang_val));
    city_btn.set_valign(gtk::Align::Center);
    city_btn.set_halign(gtk::Align::End);
    city_btn.set_hexpand(false);
    city_btn.set_vexpand(false);
    let city_tx = loc_tx.clone();

    let city_row_clone = city_row.clone();
    let lang_val_city = lang_val.clone();
    let perform_search = Rc::new(move || {
        let query = city_row_clone.text().to_string();
        if query.trim().is_empty() {
            return;
        }

        city_row_clone.remove_css_class("error");
        city_row_clone.remove_css_class("success");

        let tx = city_tx.clone();
        let city_row_for_update = city_row_clone.clone();
        let status_row_clone = status_row_clone.clone();
        let lang_val_clone = lang_val_city.clone();

        gtk::glib::spawn_future_local(async move {
            let result = location::search_city(&query, &lang_val_clone).await;
            match result {
                Ok((lat, lon, name, _timezone)) => {
                    let _ = tx.send((lat, lon, Some(name.clone())));
                    city_row_for_update.set_text(&location::short_city_with_country(&name));
                    city_row_for_update.add_css_class("success");
                    status_row_clone.set_visible(false);
                }
                Err(e) => {
                    log::error!("City search failed: {}", e);
                    city_row_for_update.add_css_class("error");
                    status_row_clone
                        .set_subtitle(&tr("City not found. Please try again.", &lang_val_clone));
                    status_row_clone.set_visible(true);
                }
            }
        });
    });

    let search_fn = perform_search.clone();
    city_row.connect_entry_activated(move |row| {
        search_fn();
        finish_entry_row_interaction(row);
    });

    let search_fn_btn = perform_search.clone();
    city_btn.connect_clicked(move |_| {
        search_fn_btn();
    });

    city_row.add_suffix(&city_btn);

    let auto_row = adw::ActionRow::builder()
        .title(tr("Auto Detection", &lang_val))
        .build();
    if let Some(name) = &config.borrow().city_name {
        auto_row.set_subtitle(&location::short_city_with_country(name));
    }
    let auto_btn = Button::with_label(&tr("Update Now", &lang_val));
    auto_btn.set_valign(gtk::Align::Center);
    auto_btn.set_halign(gtk::Align::End);
    auto_btn.set_hexpand(false);
    auto_btn.set_vexpand(false);

    let auto_tx = loc_tx.clone();
    let auto_row_clone = auto_row.clone();
    let status_row_auto = status_row_clone2;
    let lang_val_auto = lang_val.clone();

    auto_btn.connect_clicked(move |_| {
        auto_row_clone.remove_css_class("error");
        auto_row_clone.remove_css_class("success");
        status_row_auto.set_visible(false);

        let tx = auto_tx.clone();
        let auto_row_for_update = auto_row_clone.clone();
        let status_for_update = status_row_auto.clone();
        let lang_for_update = lang_val_auto.clone();

        gtk::glib::spawn_future_local(async move {
            let result = location::fetch_auto_location(&lang_for_update).await;
            match result {
                Ok((lat, lon, name)) => {
                    let _ = tx.send((lat, lon, Some(name.clone())));
                    auto_row_for_update.set_subtitle(&location::short_city_with_country(&name));
                    auto_row_for_update.add_css_class("success");
                }
                Err(e) => {
                    log::error!("Auto-location failed: {}", e);
                    auto_row_for_update.add_css_class("error");
                    status_for_update.set_subtitle(&tr(&e, &lang_for_update));
                    status_for_update.set_visible(true);
                }
            }
        });
    });

    auto_row.add_suffix(&auto_btn);

    let source_items = [
        tr("Calculated (Offline)", &lang_val),
        tr("Connected Mosque (URL)", &lang_val),
    ];
    let source_refs: Vec<&str> = source_items.iter().map(|s| s.as_str()).collect();
    let source_model = StringList::new(&source_refs);
    let source_row = ComboRow::builder()
        .title(tr("Prayer Times Source", &lang_val))
        .model(&source_model)
        .build();
    source_row.set_selected(match config.borrow().prayer_times_source {
        PrayerTimesSource::Calculated => 0,
        PrayerTimesSource::Mawaqit => 1,
    });
    location_group.add(&source_row);

    let url_row = adw::EntryRow::builder()
        .title(tr("Connected Mosque URL (mawaqit.net)", &lang_val))
        .visible(config.borrow().prayer_times_source == PrayerTimesSource::Mawaqit)
        .build();
    if let Some(url) = &config.borrow().mawaqit_url {
        url_row.set_text(url);
    } else if let Some(cache) = config.borrow().mawaqit_cache.as_ref() {
        url_row.set_text(&cache.url);
    }
    location_group.add(&url_row);

    let auto_refresh_row = adw::SwitchRow::builder()
        .title(tr("Auto refresh daily", &lang_val))
        .subtitle(tr(
            "Refresh mosque prayer times once per day while the app is open.",
            &lang_val,
        ))
        .visible(config.borrow().prayer_times_source == PrayerTimesSource::Mawaqit)
        .build();
    auto_refresh_row.set_active(config.borrow().mawaqit_auto_refresh_daily);
    location_group.add(&auto_refresh_row);

    let mawaqit_status_row = adw::ActionRow::builder()
        .title(tr("Connected Mosque", &lang_val))
        .visible(config.borrow().prayer_times_source == PrayerTimesSource::Mawaqit)
        .build();
    if let Some(cache) = config.borrow().mawaqit_cache.as_ref() {
        let title = cache
            .mosque_name
            .clone()
            .unwrap_or_else(|| cache.url.clone());
        let tz = cache.timezone.clone().unwrap_or_default();
        let tz_label = if tz.is_empty() {
            String::new()
        } else {
            location::localized_time_zone_label(&tz, &lang_val)
        };
        let subtitle = if tz_label.is_empty() {
            format!("{} • {}", tr("Last updated", &lang_val), cache.fetched_on)
        } else {
            format!(
                "{} • {} • {}",
                tz_label,
                tr("Last updated", &lang_val),
                cache.fetched_on
            )
        };
        mawaqit_status_row.set_subtitle(&subtitle);
        mawaqit_status_row.set_title(&title);
    } else {
        mawaqit_status_row.set_subtitle(&tr("Not configured", &lang_val));
    }

    let refresh_btn = Button::with_label(&tr("Refresh now", &lang_val));
    refresh_btn.set_valign(gtk::Align::Center);
    refresh_btn.set_halign(gtk::Align::End);
    mawaqit_status_row.add_suffix(&refresh_btn);
    location_group.add(&mawaqit_status_row);

    location_group.add(&mode_row);
    location_group.add(&lat_row);
    location_group.add(&lon_row);
    location_group.add(&city_row);
    location_group.add(&auto_row);
    location_group.add(&status_row);

    let config_for_auto = config.clone();
    auto_refresh_row.connect_active_notify(move |row| {
        config_for_auto.borrow_mut().mawaqit_auto_refresh_daily = row.is_active();
        AppConfig::save_shared(&config_for_auto);
    });

    let config_for_source = config.clone();
    let list_box_for_source = list_box_rc.clone();
    let url_row_for_source = url_row.clone();
    let auto_row_for_source = auto_refresh_row.clone();
    let status_for_source = mawaqit_status_row.clone();
    let refresh_btn_for_source = refresh_btn.clone();
    source_row.connect_selected_notify(move |row| {
        let show = row.selected() == 1;
        config_for_source.borrow_mut().prayer_times_source = if show {
            PrayerTimesSource::Mawaqit
        } else {
            PrayerTimesSource::Calculated
        };
        AppConfig::save_shared(&config_for_source);
        url_row_for_source.set_visible(show);
        auto_row_for_source.set_visible(show);
        status_for_source.set_visible(show);
        refresh_btn_for_source.set_visible(show);
        refresh_prayers(&config_for_source.borrow(), &list_box_for_source);
    });

    let config_for_fetch = config.clone();
    let list_box_for_fetch = list_box_rc.clone();
    let status_for_fetch = mawaqit_status_row.clone();
    let url_row_for_fetch = url_row.clone();
    let lang_for_fetch = lang_val.clone();
    let loc_tx_for_fetch = loc_tx.clone();
    let settings_box_for_fetch = settings_box.clone();
    let list_box_for_fetch_ui = list_box_rc.clone();
    let window_for_fetch = window.clone();
    let current_lang_for_fetch = current_lang.clone();
    let refresh_calendar_for_fetch = refresh_calendar.clone();
    let lang_row_for_fetch = lang_row.map(|r| std::rc::Rc::new(r.clone()));
    let do_fetch = Rc::new(move || {
        let lang_row_for_fetch_clone = lang_row_for_fetch.clone();
        let raw = url_row_for_fetch.text().to_string();
        if raw.trim().is_empty() {
            status_for_fetch.set_subtitle(&tr("Invalid Mawaqit URL", &lang_for_fetch));
            status_for_fetch.add_css_class("error");
            return;
        }
        status_for_fetch.remove_css_class("error");
        status_for_fetch.set_subtitle(&tr("Fetching...", &lang_for_fetch));
        let cfg = config_for_fetch.clone();
        let list_box = list_box_for_fetch.clone();
        let status = status_for_fetch.clone();
        let lang = lang_for_fetch.clone();
        let tx = loc_tx_for_fetch.clone();
        let settings_box = settings_box_for_fetch.clone();
        let list_box_ui = list_box_for_fetch_ui.clone();
        let window = window_for_fetch.clone();
        let current_lang = current_lang_for_fetch.clone();
        let refresh_calendar = refresh_calendar_for_fetch.clone();
        gtk::glib::spawn_future_local(async move {
            match crate::mawaqit::fetch_mawaqit_cache(&raw).await {
                Ok(cache) => {
                    let mut maybe_loc_update: Option<(f64, f64, Option<String>)> = None;
                    {
                        let mut c = cfg.borrow_mut();
                        c.mawaqit_url = Some(cache.url.clone());
                        c.mawaqit_cache = Some(cache.clone());
                        if let (Some(lat), Some(lon)) = (cache.latitude, cache.longitude) {
                            c.latitude = lat;
                            c.longitude = lon;
                            c.location_mode = LocationMode::City;
                            let fallback_city = crate::location::localized_mawaqit_city_name(
                                None,
                                cache.timezone.as_deref(),
                                cache.mosque_name.as_deref(),
                                &lang,
                            );
                            if let Some(city) = fallback_city.clone() {
                                c.city_name = Some(city.clone());
                                maybe_loc_update = Some((lat, lon, Some(city)));
                            } else {
                                maybe_loc_update = Some((lat, lon, None));
                            }
                        }

                        if let Some(ref tz) = cache.timezone {
                            if let Some(ref sys_tz) = crate::location::system_time_zone_id() {
                                if !tz.eq_ignore_ascii_case(&sys_tz) {
                                    c.timezone_mode = TimezoneMode::Named(tz.clone());
                                    log::info!(
                                        "Timezone auto-updated to {} (Mawaqit, different from system {})",
                                        tz,
                                        sys_tz
                                    );
                                }
                            }
                        }
                        c.sync_quran_state_from_disk();
                        c.save();
                    }
                    if let Some((lat, lon, None)) = &maybe_loc_update {
                        let lat = *lat;
                        let lon = *lon;
                        let cfg2 = cfg.clone();
                        let tx2 = tx.clone();
                        let lang2 = lang.clone();
                        gtk::glib::spawn_future_local(async move {
                            if let Ok(name) =
                                crate::location::resolve_city_name(lat, lon, &lang2).await
                            {
                                {
                                    let mut c = cfg2.borrow_mut();
                                    c.city_name = Some(name.clone());
                                    c.sync_quran_state_from_disk();
                                    c.save();
                                }
                                let _ = tx2.send((lat, lon, Some(name)));
                            }
                        });
                    }
                    if let Some((lat, lon, name)) = maybe_loc_update {
                        let _ = tx.send((lat, lon, name));
                    }
                    let title = cache
                        .mosque_name
                        .clone()
                        .unwrap_or_else(|| cache.url.clone());
                    let tz = cache.timezone.clone().unwrap_or_default();
                    let tz_label = if tz.is_empty() {
                        String::new()
                    } else {
                        location::localized_time_zone_label(&tz, &lang)
                    };
                    let subtitle = if tz_label.is_empty() {
                        format!("{} • {}", tr("Last updated", &lang), cache.fetched_on)
                    } else {
                        format!(
                            "{} • {} • {}",
                            tz_label,
                            tr("Last updated", &lang),
                            cache.fetched_on
                        )
                    };
                    status.set_title(&title);
                    status.set_subtitle(&subtitle);
                    status.remove_css_class("error");
                    refresh_prayers(&cfg.borrow(), &list_box);
                    setup_settings_ui(SettingsUiParams {
                        settings_box: &settings_box,
                        config: cfg.clone(),
                        list_box_rc: list_box_ui,
                        window: &window,
                        current_lang,
                        loc_tx: tx,
                        refresh_calendar,
                        lang_row: lang_row_for_fetch_clone.as_ref().map(|rc| rc.as_ref()),
                    });
                }
                Err(e) => {
                    status.add_css_class("error");
                    status.set_subtitle(&tr(&e, &lang));
                }
            }
        });
    });

    let do_fetch_btn = do_fetch.clone();
    refresh_btn.connect_clicked(move |_| {
        do_fetch_btn();
    });
    let do_fetch_entry = do_fetch.clone();
    url_row.connect_entry_activated(move |row| {
        do_fetch_entry();
        finish_entry_row_interaction(row);
    });

    let travel_group = PreferencesGroup::builder()
        .title(gtk::glib::markup_escape_text(&tr(
            "Timezone & Travel",
            &lang_val,
        )))
        .description(tr(
            "Override the timezone for prayer time calculations.",
            &lang_val,
        ))
        .build();
    travel_group.set_margin_top(12);
    travel_group.set_margin_bottom(24);
    settings_box.append(&travel_group);

    let tz_mode_strings = [
        tr("Automatic (System)", &lang_val),
        tr("Custom Timezone (IANA)", &lang_val),
        tr("Manual UTC Offset", &lang_val),
    ];
    let tz_mode_slices: Vec<&str> = tz_mode_strings.iter().map(|s| s.as_str()).collect();
    let tz_modes = StringList::new(&tz_mode_slices);
    let tz_mode_row = ComboRow::builder()
        .title(tr("Timezone Mode", &lang_val))
        .subtitle(tr(
            "How prayer times are adjusted for your timezone.",
            &lang_val,
        ))
        .model(&tz_modes)
        .build();

    let current_tz_mode = config.borrow().timezone_mode.clone();
    let tz_init_selected = match &current_tz_mode {
        TimezoneMode::Auto => 0u32,
        TimezoneMode::Named(_) => 1,
        TimezoneMode::UtcOffset(_) => 2,
    };
    tz_mode_row.set_selected(tz_init_selected);
    travel_group.add(&tz_mode_row);

    let tz_named_init = match &current_tz_mode {
        TimezoneMode::Named(s) => s.clone(),
        _ => location::system_time_zone_id().unwrap_or_default(),
    };
    let tz_named_row = adw::EntryRow::builder()
        .title(tr("IANA Timezone", &lang_val))
        .text(&tz_named_init)
        .show_apply_button(false)
        .visible(tz_init_selected == 1)
        .build();
    tz_named_row.set_input_hints(gtk::InputHints::NO_SPELLCHECK);
    tz_named_row.set_direction(gtk::TextDirection::Ltr);
    tz_named_row.add_prefix(&gtk::Image::from_icon_name("mark-location-symbolic"));
    travel_group.add(&tz_named_row);

    let update_tz_named_validation = Rc::new({
        let tz_named_row = tz_named_row.clone();
        let lang_val = lang_val.clone();
        move |text: &str, keep_success_state: bool| {
            tz_named_row.remove_css_class("error");
            tz_named_row.remove_css_class("success");

            if let Some(name) = location::validated_time_zone_id(text) {
                if keep_success_state && !text.trim().is_empty() {
                    tz_named_row.add_css_class("success");
                }
                tz_named_row
                    .set_tooltip_text(Some(&location::localized_time_zone_label(&name, &lang_val)));
            } else if text.trim().is_empty() {
                tz_named_row.set_tooltip_text(None);
            } else {
                tz_named_row.add_css_class("error");
                tz_named_row.set_tooltip_text(None);
            }
        }
    });
    update_tz_named_validation(&tz_named_init, false);

    let tz_adj = gtk::Adjustment::new(0.0, -12.0, 14.0, 0.5, 0.0, 0.0);
    if let TimezoneMode::UtcOffset(mins) = &current_tz_mode {
        tz_adj.set_value(*mins as f64 / 60.0);
    }
    let tz_offset_row = adw::SpinRow::builder()
        .title(tr("UTC Offset (hours)", &lang_val))
        .subtitle(tr("Example: +2.0 for UTC+2, -5.0 for UTC-5", &lang_val))
        .adjustment(&tz_adj)
        .digits(1)
        .visible(tz_init_selected == 2)
        .build();
    travel_group.add(&tz_offset_row);

    let tz_named_vis = tz_named_row.clone();
    let tz_offset_vis = tz_offset_row.clone();
    let config_tz_mode = config.clone();
    let list_box_tz = list_box_rc.clone();
    let tz_adj_for_mode = tz_adj.clone();
    let update_tz_named_for_mode = update_tz_named_validation.clone();
    tz_mode_row.connect_selected_notify(move |combo| {
        let sel = combo.selected();
        tz_named_vis.set_visible(sel == 1);
        tz_offset_vis.set_visible(sel == 2);
        let tz_named_text = tz_named_vis.text().to_string();
        let existing_named = match config_tz_mode.borrow().timezone_mode.clone() {
            TimezoneMode::Named(name) if !name.trim().is_empty() => Some(name),
            _ => None,
        };
        let new_mode = match sel {
            1 => {
                if let Some(name) = location::validated_time_zone_id(&tz_named_text) {
                    tz_named_vis.set_text(&name);
                    TimezoneMode::Named(name)
                } else if let Some(name) = existing_named {
                    TimezoneMode::Named(name)
                } else {
                    TimezoneMode::Auto
                }
            }
            2 => TimezoneMode::UtcOffset((tz_adj_for_mode.value() * 60.0) as i32),
            _ => TimezoneMode::Auto,
        };
        config_tz_mode.borrow_mut().timezone_mode = new_mode;
        AppConfig::save_shared(&config_tz_mode);
        refresh_prayers(&config_tz_mode.borrow(), &list_box_tz);
        update_tz_named_for_mode(&tz_named_vis.text(), sel == 1 && tz_named_vis.has_focus());
    });

    let update_tz_named_for_change = update_tz_named_validation.clone();
    tz_named_row.connect_changed(move |row| {
        let text = row.text().to_string();
        update_tz_named_for_change(&text, row.has_focus());
    });

    let config_tz_named = config.clone();
    let list_box_tz_named = list_box_rc.clone();
    let tz_mode_row_for_apply = tz_mode_row.clone();
    let tz_named_row_for_apply = tz_named_row.clone();
    let update_tz_named_for_apply = update_tz_named_validation.clone();
    let apply_named_timezone = Rc::new(move || {
        if tz_mode_row_for_apply.selected() != 1 {
            return;
        }
        let raw_text = tz_named_row_for_apply.text().to_string();
        update_tz_named_for_apply(&raw_text, tz_named_row_for_apply.has_focus());

        if let Some(name) = location::validated_time_zone_id(&raw_text) {
            tz_named_row_for_apply.set_text(&name);
            update_tz_named_for_apply(&name, tz_named_row_for_apply.has_focus());
            config_tz_named.borrow_mut().timezone_mode = TimezoneMode::Named(name);
            AppConfig::save_shared(&config_tz_named);
            refresh_prayers(&config_tz_named.borrow(), &list_box_tz_named);
        }
    });

    let apply_named_timezone_from_enter = apply_named_timezone.clone();
    tz_named_row.connect_entry_activated(move |row| {
        apply_named_timezone_from_enter();
        finish_entry_row_interaction(row);
    });

    let apply_named_timezone_on_blur = apply_named_timezone.clone();
    let update_tz_named_for_focus = update_tz_named_validation.clone();
    tz_named_row.connect_has_focus_notify(move |row| {
        if !row.has_focus() {
            apply_named_timezone_on_blur();
        }
        let text = row.text().to_string();
        update_tz_named_for_focus(&text, row.has_focus());
    });

    let config_tz_offset = config.clone();
    let list_box_tz_offset = list_box_rc.clone();
    tz_adj.connect_value_changed(move |adj| {
        if let TimezoneMode::UtcOffset(_) = config_tz_offset.borrow().timezone_mode {
            config_tz_offset.borrow_mut().timezone_mode =
                TimezoneMode::UtcOffset((adj.value() * 60.0) as i32);
            AppConfig::save_shared(&config_tz_offset);
            refresh_prayers(&config_tz_offset.borrow(), &list_box_tz_offset);
        }
    });

    let calc_group = PreferencesGroup::builder()
        .title(tr("Calculation", &lang_val))
        .build();
    calc_group.set_margin_top(12);
    calc_group.set_margin_bottom(24);
    settings_box.append(&calc_group);

    let hijri_adj = gtk::Adjustment::new(
        config.borrow().hijri_offset as f64,
        -2.0,
        2.0,
        1.0,
        0.0,
        0.0,
    );
    let hijri_row = adw::SpinRow::builder()
        .title(tr("Hijri Date Correction", &lang_val))
        .subtitle(tr("Adjust Hijri date by +/- days", &lang_val))
        .adjustment(&hijri_adj)
        .digits(0)
        .build();

    let config_hijri = config.clone();
    let refresh_calendar_hijri = refresh_calendar.clone();
    hijri_adj.connect_value_changed(move |adj| {
        config_hijri.borrow_mut().hijri_offset = adj.value() as i64;
        AppConfig::save_shared(&config_hijri);
        refresh_calendar_hijri();
    });
    calc_group.add(&hijri_row);

    let methods_strings = [
        tr("MWL", &lang_val),
        tr("ISNA", &lang_val),
        tr("Egypt", &lang_val),
        tr("Makkah", &lang_val),
        tr("Karachi", &lang_val),
        tr("Dubai", &lang_val),
        tr("MoonsightingCommittee", &lang_val),
        tr("Kuwait", &lang_val),
        tr("Qatar", &lang_val),
        tr("Singapore", &lang_val),
        tr("Turkey", &lang_val),
    ];
    let methods_slices: Vec<&str> = methods_strings.iter().map(|s| s.as_str()).collect();
    let methods = StringList::new(&methods_slices);
    let method_row = ComboRow::builder()
        .title(tr("Calculation Method", &lang_val))
        .model(&methods)
        .build();

    let current_method = config.borrow().method.clone();
    method_row.set_selected(match current_method {
        CalculationMethod::MWL => 0,
        CalculationMethod::ISNA => 1,
        CalculationMethod::Egypt => 2,
        CalculationMethod::Makkah => 3,
        CalculationMethod::Karachi => 4,
        CalculationMethod::Dubai => 5,
        CalculationMethod::MoonsightingCommittee => 6,
        CalculationMethod::Kuwait => 7,
        CalculationMethod::Qatar => 8,
        CalculationMethod::Singapore => 9,
        CalculationMethod::Turkey => 10,
    });

    let config_method = config.clone();
    let list_box_method = list_box_rc.clone();
    method_row.connect_selected_notify(move |combo| {
        let method = match combo.selected() {
            0 => CalculationMethod::MWL,
            1 => CalculationMethod::ISNA,
            2 => CalculationMethod::Egypt,
            3 => CalculationMethod::Makkah,
            4 => CalculationMethod::Karachi,
            5 => CalculationMethod::Dubai,
            6 => CalculationMethod::MoonsightingCommittee,
            7 => CalculationMethod::Kuwait,
            8 => CalculationMethod::Qatar,
            9 => CalculationMethod::Singapore,
            10 => CalculationMethod::Turkey,
            _ => CalculationMethod::MWL,
        };
        config_method.borrow_mut().method = method;
        AppConfig::save_shared(&config_method);
        refresh_prayers(&config_method.borrow(), &list_box_method);
    });
    calc_group.add(&method_row);

    let lat_row_clone = lat_row.clone();
    let lon_row_clone = lon_row.clone();
    let city_row_clone = city_row.clone();
    let auto_row_clone = auto_row.clone();

    let update_visibility = Rc::new(move |mode: &LocationMode| {
        lat_row_clone.set_visible(*mode == LocationMode::Manual);
        lon_row_clone.set_visible(*mode == LocationMode::Manual);
        city_row_clone.set_visible(*mode == LocationMode::City);
        auto_row_clone.set_visible(*mode == LocationMode::Auto);
    });

    update_visibility(&current_mode);

    let update_vis_clone = update_visibility.clone();
    let config_mode = config.clone();
    let list_box_mode = list_box_rc.clone();
    let source_row_for_mode = source_row.clone();
    let url_row_for_mode = url_row.clone();
    let auto_row_for_mode = auto_refresh_row.clone();
    let status_row_for_mode = mawaqit_status_row.clone();
    let refresh_btn_for_mode = refresh_btn.clone();
    mode_row.connect_selected_notify(move |combo| {
        let mode = match combo.selected() {
            0 => LocationMode::Manual,
            1 => LocationMode::City,
            2 => LocationMode::Auto,
            _ => LocationMode::Manual,
        };
        let was_mawaqit = config_mode.borrow().prayer_times_source == PrayerTimesSource::Mawaqit;
        {
            let mut c = config_mode.borrow_mut();
            if was_mawaqit {
                c.prayer_times_source = PrayerTimesSource::Calculated;
            }
            c.location_mode = mode.clone();
        }
        if was_mawaqit {
            source_row_for_mode.set_selected(0);
            url_row_for_mode.set_visible(false);
            auto_row_for_mode.set_visible(false);
            status_row_for_mode.set_visible(false);
            refresh_btn_for_mode.set_visible(false);
        }
        AppConfig::save_shared(&config_mode);
        update_vis_clone(&mode);
        refresh_prayers(&config_mode.borrow(), &list_box_mode);
    });

    let madhab_strings = [
        tr("Shafi (Standard/Maliki/Hanbali)", &lang_val),
        tr("Hanafi", &lang_val),
    ];
    let madhab_slices: Vec<&str> = madhab_strings.iter().map(|s| s.as_str()).collect();
    let madhabs = StringList::new(&madhab_slices);
    let madhab_row = ComboRow::builder()
        .title(tr("Asr Calculation (Madhab)", &lang_val))
        .model(&madhabs)
        .build();

    let current_madhab = config.borrow().madhab.clone();
    if current_madhab == MadhabChoice::Hanafi {
        madhab_row.set_selected(1);
    } else {
        madhab_row.set_selected(0);
    }

    let config_madhab = config.clone();
    let list_box_madhab = list_box_rc.clone();
    madhab_row.connect_selected_notify(move |combo| {
        let index = combo.selected();
        let m = if index == 1 {
            MadhabChoice::Hanafi
        } else {
            MadhabChoice::Shafi
        };
        config_madhab.borrow_mut().madhab = m;
        AppConfig::save_shared(&config_madhab);
        refresh_prayers(&config_madhab.borrow(), &list_box_madhab);
    });
    calc_group.add(&madhab_row);

    let note_row = adw::ActionRow::builder()
        .title(tr("Note", &lang_val))
        .subtitle(tr(
            "Maliki/Hanbali use Standard (Shafi) for Asr.",
            &lang_val,
        ))
        .build();
    calc_group.add(&note_row);

    let iqamah_group = PreferencesGroup::builder()
        .title(tr("Iqamah Delays", &lang_val))
        .description(tr(
            "Minutes to wait after the Adhan before the Iqamah (second call to prayer).",
            &lang_val,
        ))
        .build();
    iqamah_group.set_margin_top(12);
    iqamah_group.set_margin_bottom(24);
    settings_box.append(&iqamah_group);

    append_settings_section_heading(
        settings_box,
        &tr("Notifications & Audio", &lang_val),
        Some(&tr(
            "Choose when and how you receive prayer reminders and the Adhan sound.",
            &lang_val,
        )),
        24,
    );

    let notif_group = PreferencesGroup::new();
    notif_group.set_margin_top(0);
    notif_group.set_margin_bottom(12);
    settings_box.append(&notif_group);

    let notify_toggle = adw::SwitchRow::builder()
        .title(tr("Pre-Prayer Alert", &lang_val))
        .subtitle(tr("Get notified before the prayer time.", &lang_val))
        .build();
    notify_toggle.set_active(config.borrow().pre_prayer_notify);

    let config_notify = config.clone();
    notify_toggle.connect_active_notify(move |row| {
        config_notify.borrow_mut().pre_prayer_notify = row.is_active();
        AppConfig::save_shared(&config_notify);
    });
    notif_group.add(&notify_toggle);

    let notify_time = adw::SpinRow::builder()
        .title(tr("Alert Time", &lang_val))
        .subtitle(tr("Minutes before prayer", &lang_val))
        .adjustment(&gtk::Adjustment::new(
            config.borrow().pre_prayer_minutes as f64,
            1.0,
            60.0,
            1.0,
            5.0,
            0.0,
        ))
        .digits(0)
        .build();

    let config_time = config.clone();
    notify_time.adjustment().connect_value_changed(move |adj| {
        config_time.borrow_mut().pre_prayer_minutes = adj.value() as u32;
        AppConfig::save_shared(&config_time);
    });
    notif_group.add(&notify_time);

    let time_row_clone = notify_time.clone();
    notify_toggle.connect_active_notify(move |row| {
        time_row_clone.set_visible(row.is_active());
    });
    notify_time.set_visible(config.borrow().pre_prayer_notify);

    let test_notify_btn = Button::builder()
        .label(tr("Test Notification", &lang_val))
        .margin_top(12)
        .build();

    let config_test_notif = config.clone();
    let current_lang_notif = current_lang.clone();
    bind_audio_toggle_button_sync(&test_notify_btn, current_lang.clone(), "Test Notification");
    test_notify_btn.connect_clicked(move |btn| {
        let lang = current_lang_notif.borrow().clone();
        if crate::audio::is_playing() {
            crate::audio::stop();
            set_audio_toggle_button_label(btn, &lang, "Test Notification", false);
        } else {
            let cfg = config_test_notif.borrow();
            notifications::show_notification(
                &tr("It's time for", &lang),
                &tr(
                    "This is a test notification from Khushu. May your prayers be accepted.",
                    &lang,
                ),
                true,
                &tr("Open Khushu", &lang),
                &tr("Stop Adhan", &lang),
            );
            if !cfg.adhan_muted {
                let path = cfg
                    .adhan_sound_path
                    .clone()
                    .unwrap_or_else(|| "assets/audio/Madinah.mp3".to_string());
                crate::audio::play_adhan(&path, cfg.adhan_volume);
                set_audio_toggle_button_label(btn, &lang, "Test Notification", true);
            }
        }
    });

    notif_group.add(&test_notify_btn);

    let prayer_iqamah_defs = [
        ("Fajr", 20u32),
        ("Dhuhr", 10u32),
        ("Asr", 10u32),
        ("Maghrib", 5u32),
        ("Isha", 10u32),
    ];

    for (prayer_name, default_mins) in prayer_iqamah_defs {
        let current = config
            .borrow()
            .iqamah_minutes
            .get(prayer_name)
            .copied()
            .unwrap_or(default_mins);
        let iq_adj = gtk::Adjustment::new(current as f64, 0.0, 60.0, 1.0, 5.0, 0.0);
        let iq_row = adw::SpinRow::builder()
            .title(tr(prayer_name, &lang_val))
            .subtitle(tr("Minutes", &lang_val))
            .adjustment(&iq_adj)
            .digits(0)
            .build();
        iqamah_group.add(&iq_row);

        let config_iq = config.clone();
        let prayer_key = prayer_name.to_string();
        iq_adj.connect_value_changed(move |adj| {
            config_iq
                .borrow_mut()
                .iqamah_minutes
                .insert(prayer_key.clone(), adj.value() as u32);
            AppConfig::save_shared(&config_iq);
        });
    }

    let audio_group = PreferencesGroup::new();
    audio_group.set_margin_bottom(12);
    settings_box.append(&audio_group);

    let preset_files: Vec<String> = vec!["Madinah.mp3".to_string(), "Makkah.mp3".to_string()];

    let mut preset_labels: Vec<String> = Vec::new();
    preset_labels.push(tr("Default", &lang_val));
    preset_labels.push(tr("Custom File...", &lang_val));
    for name in &preset_files {
        preset_labels.push(adhan_preset_label(name, &lang_val));
    }

    let label_refs: Vec<&str> = preset_labels.iter().map(|s| s.as_str()).collect();
    let model = gtk::StringList::new(&label_refs);

    let sound_combo = ComboRow::builder()
        .title(tr("Adhan Sound", &lang_val))
        .model(&model)
        .build();

    let current_path = config.borrow().adhan_sound_path.clone();
    if let Some(path) = current_path {
        let path_obj = PathBuf::from(&path);
        if let Some(name) = path_obj.file_name().and_then(|n| n.to_str()) {
            if let Some(pos) = preset_files.iter().position(|p| p.as_str() == name) {
                sound_combo.set_selected((pos + 2) as u32);
            } else {
                sound_combo.set_selected(1);
                sound_combo.set_subtitle(&path);
            }
        } else {
            sound_combo.set_selected(1);
            sound_combo.set_subtitle(&path);
        }
    } else {
        sound_combo.set_selected(0);
        sound_combo.set_subtitle(&tr("Using builtin default", &lang_val));
    }

    let window_clone_sound = window.clone();
    let config_sound = config.clone();
    let preset_files_clone = preset_files.clone();
    let lang_for_audio = lang_val.clone();

    sound_combo.connect_selected_notify(move |combo| {
        let index = combo.selected() as usize;

        if index == 0 {
            config_sound.borrow_mut().adhan_sound_path = None;
            AppConfig::save_shared(&config_sound);
            combo.set_subtitle(&tr("Using builtin default", &lang_for_audio));
        } else if index == 1 {
            let file_filter = gtk::FileFilter::new();
            file_filter.set_name(Some(&tr("Audio Files", &lang_for_audio)));
            file_filter.add_mime_type("audio/mpeg");
            file_filter.add_mime_type("audio/mp3");
            file_filter.add_mime_type("audio/ogg");

            let filters = gtk::gio::ListStore::new::<gtk::FileFilter>();
            filters.append(&file_filter);

            let dialog = gtk::FileDialog::builder()
                .title(tr("Select Adhan Sound", &lang_for_audio))
                .modal(true)
                .filters(&filters)
                .build();

            let config_dialog = config_sound.clone();
            let combo_dialog = combo.clone();
            let parent_window = window_clone_sound.clone();

            gtk::glib::spawn_future_local(async move {
                if let Ok(file) = dialog.open_future(Some(&parent_window)).await
                    && let Some(path) = file.path()
                    && let Some(path_str) = path.to_str()
                {
                    config_dialog.borrow_mut().adhan_sound_path = Some(path_str.to_string());
                    AppConfig::save_shared(&config_dialog);
                    combo_dialog.set_subtitle(path_str);
                }
            });
        } else {
            let mut path = PathBuf::from("assets/audio");
            let file_name = &preset_files_clone[index - 2];
            path.push(file_name);
            if let Some(path_str) = path.to_str() {
                config_sound.borrow_mut().adhan_sound_path = Some(path_str.to_string());
                AppConfig::save_shared(&config_sound);
                combo.set_subtitle(path_str);
            }
        }
    });

    audio_group.add(&sound_combo);

    let mute_toggle = adw::SwitchRow::builder()
        .title(tr("Mute Adhan", &lang_val))
        .subtitle(tr("Silence the Adhan sound at prayer time.", &lang_val))
        .build();
    mute_toggle.set_active(config.borrow().adhan_muted);
    let config_mute = config.clone();
    mute_toggle.connect_active_notify(move |row| {
        config_mute.borrow_mut().adhan_muted = row.is_active();
        AppConfig::save_shared(&config_mute);
    });
    audio_group.add(&mute_toggle);

    let volume_adj = gtk::Adjustment::new(
        (config.borrow().adhan_volume * 100.0) as f64,
        0.0,
        100.0,
        5.0,
        10.0,
        0.0,
    );
    let volume_row = adw::SpinRow::builder()
        .title(tr("Adhan Volume", &lang_val))
        .subtitle(tr("Volume level (0–100%)", &lang_val))
        .adjustment(&volume_adj)
        .digits(0)
        .build();
    volume_row.set_visible(!config.borrow().adhan_muted);

    let config_vol = config.clone();
    volume_adj.connect_value_changed(move |adj| {
        config_vol.borrow_mut().adhan_volume = (adj.value() / 100.0) as f32;
        AppConfig::save_shared(&config_vol);
    });
    audio_group.add(&volume_row);

    let volume_row_clone = volume_row.clone();
    mute_toggle.connect_active_notify(move |row| {
        volume_row_clone.set_visible(!row.is_active());
    });

    let test_audio_btn = Button::builder()
        .label(tr("▶ Preview Adhan", &lang_val))
        .margin_top(8)
        .build();

    let config_test = config.clone();
    let current_lang_audio = current_lang.clone();
    bind_audio_toggle_button_sync(&test_audio_btn, current_lang.clone(), "▶ Preview Adhan");
    test_audio_btn.connect_clicked(move |btn| {
        let lang = current_lang_audio.borrow().clone();
        if crate::audio::is_playing() {
            crate::audio::stop();
            set_audio_toggle_button_label(btn, &lang, "▶ Preview Adhan", false);
        } else {
            let cfg = config_test.borrow();
            if cfg.adhan_muted {
                return;
            }
            let path = cfg
                .adhan_sound_path
                .clone()
                .unwrap_or_else(|| "assets/audio/Madinah.mp3".to_string());

            crate::audio::play_adhan(&path, cfg.adhan_volume);
            set_audio_toggle_button_label(btn, &lang, "▶ Preview Adhan", true);
        }
    });
    audio_group.add(&test_audio_btn);
}

fn adhan_preset_label(file_name: &str, lang: &str) -> String {
    let stem = std::path::Path::new(file_name)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(file_name);
    match stem {
        "Makkah" => tr("Makkah Adhan", lang),
        "Madinah" => tr("Madinah Adhan", lang),
        _ => stem.to_string(),
    }
}

pub fn refresh_prayers(config: &AppConfig, list_box: &ListBox) {
    while let Some(child) = list_box.first_child() {
        list_box.remove(&child);
    }

    let today = crate::time::effective_today(config);
    let current_lang_val = config.language.clone();

    if let Some(schedule) = crate::time::schedule_for_config(config, today) {
        let prayers = [
            ("Fajr", schedule.fajr),
            ("Sunrise", schedule.shurooq),
            ("Dhuhr", schedule.dhuhr),
            ("Asr", schedule.asr),
            ("Maghrib", schedule.maghrib),
            ("Isha", schedule.isha),
        ];

        for (name, time) in prayers {
            let row = adw::ActionRow::builder()
                .title(tr(name, &current_lang_val))
                .subtitle(time.format("%H:%M").to_string())
                .name(name)
                .build();
            list_box.append(&row);
        }
    }
}
