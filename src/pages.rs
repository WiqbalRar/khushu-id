use std::cell::RefCell;
use std::rc::Rc;

use adw::ComboRow;
use adw::prelude::*;
use gtk::{Box, Label, ListBox, Orientation, SelectionMode};
use gtk4 as gtk;
use libadwaita as adw;

use crate::adkar;
use crate::calendar;
use crate::config::AppConfig;
use crate::home_ui::refresh_home_ui;
use crate::i18n::tr;
use crate::qibla_ui;
use crate::settings_ui;

pub struct PagesParams {
    pub view_stack: Rc<adw::ViewStack>,
    pub split_view: adw::OverlaySplitView,
    pub current_lang: Rc<RefCell<String>>,
    pub config: Rc<RefCell<AppConfig>>,
    pub loc_tx: std::sync::mpsc::Sender<(f64, f64, Option<String>)>,
    pub loc_rx: std::sync::mpsc::Receiver<(f64, f64, Option<String>)>,
    pub compass_manager: Rc<crate::qibla::CompassManager>,
    pub window: adw::ApplicationWindow,
    pub sidebar_list: gtk::ListBox,
    pub window_title: adw::WindowTitle,
}

pub struct PagesContext {
    pub hero_label: Label,
    pub hijri_label: Label,
    pub location_label: Label,
    pub list_box: Rc<ListBox>,
    pub refresh_qibla: Rc<dyn Fn()>,
}

pub fn build_pages(params: PagesParams) -> PagesContext {
    let PagesParams {
        view_stack,
        split_view,
        current_lang,
        config,
        loc_tx,
        loc_rx,
        compass_manager,
        window,
        sidebar_list,
        window_title,
    } = params;
    let home_scroll = gtk::ScrolledWindow::builder()
        .vexpand(true)
        .propagate_natural_width(false)
        .propagate_natural_height(false)
        .build();

    let home_content_box = Box::new(Orientation::Vertical, 0);
    home_content_box.set_margin_top(24);
    home_content_box.set_margin_bottom(24);
    home_content_box.set_margin_start(12);
    home_content_box.set_margin_end(12);

    let home_clamp = adw::Clamp::builder()
        .maximum_size(800)
        .tightening_threshold(600)
        .child(&home_content_box)
        .build();

    home_scroll.set_child(Some(&home_clamp));

    let hero_box = Box::new(Orientation::Vertical, 8);
    hero_box.set_halign(gtk::Align::Center);
    hero_box.set_margin_top(12);
    hero_box.set_margin_bottom(12);

    let hero_label = Label::builder()
        .label("")
        .css_classes(["title-1"])
        .wrap(true)
        .justify(gtk::Justification::Center)
        .build();

    let hijri_label = Label::builder()
        .label("...")
        .css_classes(["title-3", "dim-label"])
        .wrap(true)
        .justify(gtk::Justification::Center)
        .build();

    let location_label = Label::builder()
        .label("...")
        .css_classes(["title-4", "dim-label"])
        .wrap(true)
        .justify(gtk::Justification::Center)
        .build();

    hero_box.append(&hero_label);
    hero_box.append(&hijri_label);
    hero_box.append(&location_label);
    home_content_box.append(&hero_box);

    let list_box = ListBox::builder()
        .selection_mode(SelectionMode::None)
        .css_classes(["boxed-list"])
        .margin_start(8)
        .margin_end(8)
        .margin_bottom(8)
        .build();
    let list_box_rc = Rc::new(list_box);
    home_content_box.append(list_box_rc.as_ref());

    let hijri_label_ref = hijri_label.clone();
    let location_label_ref = location_label.clone();
    let list_box_home = list_box_rc.clone();
    let config_home_ref = config.clone();
    let refresh_home = Rc::new(move || {
        let lang = config_home_ref.borrow().language.clone();
        refresh_home_ui(
            &hijri_label_ref,
            &location_label_ref,
            &lang,
            &config_home_ref.borrow(),
        );
        settings_ui::refresh_prayers(&config_home_ref.borrow(), &list_box_home);
    });
    let refresh_home_initial = refresh_home.clone();
    refresh_home_initial();

    let refresh_home_timer = refresh_home.clone();
    gtk::glib::timeout_add_local(std::time::Duration::from_secs(60), move || {
        refresh_home_timer();
        gtk::glib::ControlFlow::Continue
    });

    let config_loc = config.clone();
    let list_box_loc = list_box_rc.clone();
    let hijri_label_loc = hijri_label.clone();
    let location_label_loc = location_label.clone();
    let current_lang_loc = current_lang.clone();

    gtk::glib::timeout_add_local(std::time::Duration::from_millis(200), move || {
        while let Ok((lat, lon, city)) = loc_rx.try_recv() {
            {
                let mut cfg = config_loc.borrow_mut();
                cfg.latitude = lat;
                cfg.longitude = lon;
                if let Some(name) = city {
                    cfg.city_name = Some(name);
                }
                cfg.sync_quran_state_from_disk();
                cfg.save();
            }

            let cfg = config_loc.borrow();
            let lang = current_lang_loc.borrow();
            refresh_home_ui(&hijri_label_loc, &location_label_loc, &lang, &cfg);
            settings_ui::refresh_prayers(&cfg, &list_box_loc);
        }
        gtk::glib::ControlFlow::Continue
    });

    view_stack.add_named(&home_scroll, Some("home"));

    let (calendar_page, refresh_calendar) =
        calendar::create_calendar_page(current_lang.clone(), config.clone());

    let calendar_clamp = adw::Clamp::builder()
        .maximum_size(800)
        .tightening_threshold(600)
        .child(&calendar_page)
        .build();

    let calendar_scroll = gtk::ScrolledWindow::builder()
        .child(&calendar_clamp)
        .vexpand(true)
        .propagate_natural_width(false)
        .propagate_natural_height(false)
        .build();

    view_stack.add_named(&calendar_scroll, Some("calendar"));

    let calendar_grid = calendar_page
        .first_child()
        .and_then(|c| c.next_sibling())
        .and_then(|c| c.downcast::<gtk::Grid>().ok())
        .expect("Could not find calendar grid");

    let mut classes = calendar_grid.css_classes();
    if !classes.contains(&"compact-calendar".into()) {
        classes.push("compact-calendar".into());
    }

    let breakpoint = adw::Breakpoint::new(adw::BreakpointCondition::new_length(
        adw::BreakpointConditionLengthType::MaxWidth,
        370.0,
        adw::LengthUnit::Px,
    ));
    breakpoint.add_setter(&split_view, "collapsed", Some(&true.to_value()));
    breakpoint.add_setter(&calendar_grid, "css-classes", Some(&classes.to_value()));

    window.add_breakpoint(breakpoint);

    let qibla_page = qibla_ui::create_qibla_page(config.clone(), compass_manager.clone());

    let qibla_clamp = adw::Clamp::builder()
        .maximum_size(600)
        .tightening_threshold(400)
        .child(&qibla_page.container)
        .build();

    let qibla_scroll = gtk::ScrolledWindow::builder()
        .child(&qibla_clamp)
        .vexpand(true)
        .propagate_natural_width(false)
        .propagate_natural_height(false)
        .build();

    view_stack.add_named(&qibla_scroll, Some("qibla"));
    let refresh_qibla = qibla_page.refresh.clone();

    let view_stack_for_compass = view_stack.clone();
    let compass_for_compass = compass_manager.clone();
    view_stack.connect_visible_child_name_notify(move |_| {
        let name = view_stack_for_compass
            .visible_child_name()
            .map(|s| s.to_string())
            .unwrap_or_default();
        if name == "qibla" {
            compass_for_compass.start_monitoring();
        } else {
            compass_for_compass.stop();
        }
    });

    let (adkar_box, refresh_adkar) = adkar::create_adkar_page(config.clone());
    view_stack.add_named(&adkar_box, Some("adkar"));

    let quran_page = crate::quran::create_quran_page(&current_lang.borrow(), &view_stack);
    view_stack.add_named(&quran_page, Some("quran"));

    let settings_box = Box::new(Orientation::Vertical, 0);
    settings_box.set_margin_top(24);
    settings_box.set_margin_bottom(24);
    settings_box.set_margin_start(12);
    settings_box.set_margin_end(12);

    let settings_clamp = adw::Clamp::builder()
        .maximum_size(800)
        .tightening_threshold(600)
        .child(&settings_box)
        .build();

    let lang_model = gtk::StringList::new(&[
        &tr("System Default", &current_lang.borrow()),
        &tr("English", &current_lang.borrow()),
        &tr("Arabic", &current_lang.borrow()),
        &tr("French", &current_lang.borrow()),
        &tr("Spanish", &current_lang.borrow()),
        &tr("Turkish", &current_lang.borrow()),
    ]);
    let lang_model_rc = Rc::new(lang_model);
    let lang_row = ComboRow::builder()
        .title(tr("Language", &current_lang.borrow()))
        .model(&*lang_model_rc)
        .build();

    let dynamic_settings_box = Box::new(Orientation::Vertical, 0);
    settings_box.append(&dynamic_settings_box);

    match current_lang.borrow().as_str() {
        "en" => lang_row.set_selected(1),
        "ar" => lang_row.set_selected(2),
        "fr" => lang_row.set_selected(3),
        "es" => lang_row.set_selected(4),
        "tr" => lang_row.set_selected(5),
        _ => lang_row.set_selected(0),
    }

    let current_lang_settings = current_lang.clone();
    let config_settings = config.clone();
    let refresh_cal_settings = refresh_calendar.clone();
    let refresh_adkar_settings = refresh_adkar.clone();
    let refresh_qibla_settings = refresh_qibla.clone();
    let is_updating_lang = Rc::new(RefCell::new(false));

    let sidebar_list_lang = sidebar_list.clone();
    let lang_row_model = lang_row.clone();
    let lang_model_lang = lang_model_rc.clone();
    let window_title_lang = window_title.clone();
    let window_app_lang = window.clone();
    let view_stack_lang = view_stack.clone();
    let window_settings_closure = window.clone();
    let dynamic_settings_box_closure = dynamic_settings_box.clone();
    let list_box_rc_settings = list_box_rc.clone();
    let refresh_home_settings = refresh_home.clone();
    let loc_tx_settings = loc_tx.clone();
    let lang_row_for_settings = lang_row.clone();
    let lang_row_for_settings_closure = lang_row.clone();

    let is_updating_lang_handler = is_updating_lang.clone();
    lang_row.connect_selected_notify(move |row| {
        if *is_updating_lang_handler.borrow() {
            return;
        }
        let selected_lang;
        let mut lang_changed = false;
        {
            let mut lang = current_lang_settings.borrow_mut();
            let next_lang = match row.selected() {
                1 => "en".to_string(),
                2 => "ar".to_string(),
                3 => "fr".to_string(),
                4 => "es".to_string(),
                5 => "tr".to_string(),
                _ => "auto".to_string(),
            };
            if *lang != next_lang {
                *lang = next_lang;
                lang_changed = true;
            }
            selected_lang = lang.clone();
        }
        if !lang_changed {
            return;
        }
        let mut should_save = false;
        {
            let mut cfg = config_settings.borrow_mut();
            if cfg.language != selected_lang {
                cfg.language = selected_lang.clone();
                should_save = true;
            }
        }
        if should_save {
            AppConfig::save_shared(&config_settings);
        }

        let detected_lang = if selected_lang == "auto" || selected_lang.is_empty() {
            crate::i18n::detect_system_locale()
        } else {
            selected_lang.clone()
        };

        {
            let mut lang = current_lang_settings.borrow_mut();
            if *lang != detected_lang {
                *lang = detected_lang.clone();
            }
        }

        crate::i18n::update_locale(&detected_lang);

        if detected_lang == "ar" {
            gtk::Widget::set_default_direction(gtk::TextDirection::Rtl);
            window_app_lang.set_direction(gtk::TextDirection::Rtl);
        } else {
            gtk::Widget::set_default_direction(gtk::TextDirection::Ltr);
            window_app_lang.set_direction(gtk::TextDirection::Ltr);
        }

        crate::apply_font_css(&detected_lang);

        let style_manager = adw::StyleManager::default();
        match config_settings.borrow().theme {
            crate::config::ThemeMode::Light => {
                style_manager.set_color_scheme(adw::ColorScheme::ForceLight)
            }
            crate::config::ThemeMode::Dark => {
                style_manager.set_color_scheme(adw::ColorScheme::PreferDark)
            }
            crate::config::ThemeMode::System => {
                style_manager.set_color_scheme(adw::ColorScheme::Default)
            }
        }

        lang_row_model.set_title(&tr("Language", &detected_lang));
        let selected_index = row.selected();
        let lang_items = [
            tr("System Default", &detected_lang),
            tr("English", &detected_lang),
            tr("Arabic", &detected_lang),
            tr("French", &detected_lang),
            tr("Spanish", &detected_lang),
            tr("Turkish", &detected_lang),
        ];
        let lang_item_refs: Vec<&str> = lang_items.iter().map(|s| s.as_str()).collect();
        *is_updating_lang_handler.borrow_mut() = true;
        lang_model_lang.splice(0, lang_model_lang.n_items(), &lang_item_refs);
        lang_row_model.set_selected(selected_index);
        *is_updating_lang_handler.borrow_mut() = false;

        let mut curr = sidebar_list_lang.first_child();
        let lang_val = detected_lang.clone();
        let labels = [
            tr("Home", &lang_val),
            tr("Calendar", &lang_val),
            tr("Qibla", &lang_val),
            tr("Adkar", &lang_val),
            tr("Noble Quran", &lang_val),
            tr("Settings", &lang_val),
            tr("About", &lang_val),
        ];
        let mut idx = 0;
        while let Some(child) = curr {
            if let Some(row_container) = child.downcast_ref::<gtk::ListBoxRow>()
                && let Some(row) = row_container
                    .child()
                    .and_then(|c| c.downcast::<adw::ActionRow>().ok())
                && idx < labels.len()
            {
                row.set_title(&labels[idx]);
                idx += 1;
            }
            curr = child.next_sibling();
        }

        if let Some(name) = view_stack_lang.visible_child_name() {
            let title = match name.as_str() {
                "home" => tr("Prayer Times", &selected_lang),
                "calendar" => tr("Calendar", &selected_lang),
                "qibla" => tr("Qibla", &selected_lang),
                "adkar" => tr("Adkar", &selected_lang),
                "quran" => tr("Noble Quran", &selected_lang),
                "settings" => tr("Settings", &selected_lang),
                _ => "Khushu".to_string(),
            };
            window_title_lang.set_title(&title);
        }

        window_app_lang.set_title(Some(&tr("Khushu", &selected_lang)));

        refresh_cal_settings();
        refresh_adkar_settings();
        refresh_qibla_settings();
        refresh_home_settings();
        crate::quran::refresh_quran_ui(&view_stack_lang, &detected_lang);
        let should_refresh_city = {
            let cfg = config_settings.borrow();
            cfg.city_name.is_some()
                || (cfg.prayer_times_source == crate::config::PrayerTimesSource::Mawaqit
                    && cfg.mawaqit_cache.is_some())
        };
        if should_refresh_city {
            let tx = loc_tx_settings.clone();
            let lang = detected_lang.clone();
            let config_for_loc = config_settings.clone();
            let settings_box_for_loc = dynamic_settings_box_closure.clone();
            let list_box_for_loc = list_box_rc_settings.clone();
            let window_for_loc = window_settings_closure.clone();
            let current_lang_for_loc = current_lang_settings.clone();
            let refresh_cal_for_loc = refresh_cal_settings.clone();
            let lang_row_for_loc = lang_row_for_settings_closure.clone();
            gtk::glib::spawn_future_local(async move {
                let (lat, lon, mawaqit_city) = {
                    let cfg = config_for_loc.borrow();
                    let mawaqit_city =
                        if cfg.prayer_times_source == crate::config::PrayerTimesSource::Mawaqit {
                            cfg.mawaqit_cache.as_ref().and_then(|cache| {
                                crate::location::localized_mawaqit_city_name(
                                    cfg.city_name.as_deref(),
                                    cache.timezone.as_deref(),
                                    cache.mosque_name.as_deref(),
                                    &lang,
                                )
                            })
                        } else {
                            None
                        };
                    (cfg.latitude, cfg.longitude, mawaqit_city)
                };
                let resolved_name = if let Some(name) = mawaqit_city {
                    Some(name)
                } else {
                    crate::location::resolve_city_name(lat, lon, &lang)
                        .await
                        .ok()
                };
                if let Some(name) = resolved_name {
                    {
                        let mut cfg = config_for_loc.borrow_mut();
                        cfg.city_name = Some(name.clone());
                        cfg.sync_quran_state_from_disk();
                        cfg.save();
                    }
                    let _ = tx.send((lat, lon, Some(name)));
                    settings_ui::setup_settings_ui(settings_ui::SettingsUiParams {
                        settings_box: &settings_box_for_loc,
                        config: config_for_loc.clone(),
                        list_box_rc: list_box_for_loc.clone(),
                        window: &window_for_loc,
                        current_lang: current_lang_for_loc.clone(),
                        loc_tx: tx.clone(),
                        refresh_calendar: refresh_cal_for_loc.clone(),
                        lang_row: Some(&lang_row_for_loc),
                    });
                }
            });
        }

        while let Some(child) = dynamic_settings_box_closure.first_child() {
            dynamic_settings_box_closure.remove(&child);
        }
        settings_ui::setup_settings_ui(settings_ui::SettingsUiParams {
            settings_box: &dynamic_settings_box_closure,
            config: config_settings.clone(),
            list_box_rc: list_box_rc_settings.clone(),
            window: &window_settings_closure,
            current_lang: current_lang_settings.clone(),
            loc_tx: loc_tx_settings.clone(),
            refresh_calendar: refresh_cal_settings.clone(),
            lang_row: Some(&lang_row_for_settings_closure),
        });
    });

    settings_ui::setup_settings_ui(settings_ui::SettingsUiParams {
        settings_box: &dynamic_settings_box,
        config: config.clone(),
        list_box_rc: list_box_rc.clone(),
        window: &window,
        current_lang: current_lang.clone(),
        loc_tx: loc_tx.clone(),
        refresh_calendar: refresh_calendar.clone(),
        lang_row: Some(&lang_row_for_settings),
    });

    let settings_scroll = gtk::ScrolledWindow::builder()
        .vexpand(true)
        .propagate_natural_width(false)
        .propagate_natural_height(false)
        .child(&settings_clamp)
        .build();

    view_stack.add_named(&settings_scroll, Some("settings"));

    let toast_overlay = adw::ToastOverlay::new();
    toast_overlay.set_child(Some(view_stack.as_ref()));
    split_view.set_content(Some(&toast_overlay));

    PagesContext {
        hero_label,
        hijri_label,
        location_label,
        list_box: list_box_rc,
        refresh_qibla,
    }
}
