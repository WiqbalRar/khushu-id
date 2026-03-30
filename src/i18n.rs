use gettextrs::{LocaleCategory, bind_textdomain_codeset, bindtextdomain, dgettext, setlocale};

pub fn get_locale_dir() -> String {
    if let Some(dir) = option_env!("LOCALEDIR")
        && std::path::Path::new(dir).exists()
    {
        return dir.to_string();
    }

    if std::path::Path::new("/app/share/locale").exists() {
        return "/app/share/locale".to_string();
    }

    if let Ok(snap) = std::env::var("SNAP") {
        return format!("{}/usr/share/locale", snap);
    }

    if let Ok(canon) = std::fs::canonicalize("target/locale") {
        return canon.to_string_lossy().to_string();
    }

    "./po".to_string()
}

pub fn update_locale(lang: &str) {
    if lang == "auto" || lang.is_empty() {
        let system_lang = detect_system_locale();
        update_locale_internal(&system_lang);
    } else {
        update_locale_internal(lang);
    }
}

fn detect_system_locale() -> String {
    if let Some(lang) = std::env::var_os("LANGUAGE").and_then(|s| s.into_string().ok())
        && !lang.is_empty()
        && lang != "C"
        && lang != "POSIX"
    {
        return lang.split(':').next().unwrap_or("en").to_string();
    }

    if let Some(lang) = std::env::var_os("LC_ALL").and_then(|s| s.into_string().ok())
        && !lang.is_empty()
        && lang != "C"
        && lang != "POSIX"
    {
        return lang.split('.').next().unwrap_or("en").to_string();
    }

    if let Some(lang) = std::env::var_os("LANG").and_then(|s| s.into_string().ok())
        && !lang.is_empty()
        && lang != "C"
        && lang != "POSIX"
    {
        return lang.split('.').next().unwrap_or("en").to_string();
    }

    if let Some(actual) = setlocale(LocaleCategory::LcAll, "") {
        let locale_str = String::from_utf8_lossy(&actual);
        if !locale_str.is_empty() && locale_str != "C" && locale_str != "POSIX" {
            return locale_str.split('_').next().unwrap_or("en").to_string();
        }
    }

    "en".to_string()
}

fn update_locale_internal(lang: &str) {
    unsafe {
        std::env::set_var("LANGUAGE", lang);

        let candidates = if lang == "ar" {
            vec![
                "ar_DZ.UTF-8".to_string(),
                "ar_SA.UTF-8".to_string(),
                "ar.UTF-8".to_string(),
                "en_US.UTF-8".to_string(),
                "C.UTF-8".to_string(),
            ]
        } else {
            vec![
                format!("{}.UTF-8", lang),
                format!("{}_{}.UTF-8", lang, lang.to_uppercase()),
                "en_US.UTF-8".to_string(),
                "C.UTF-8".to_string(),
            ]
        };

        for loc in candidates {
            std::env::set_var("LC_ALL", &loc);
            std::env::set_var("LANG", &loc);

            if let Some(actual) = setlocale(LocaleCategory::LcAll, "")
                && actual != b"C"
                && actual != b"POSIX"
            {
                break;
            }
        }
    }

    let _ = setlocale(LocaleCategory::LcAll, "");

    let locale_dir = get_locale_dir();
    let gettext_package = option_env!("GETTEXT_PACKAGE").unwrap_or("khushu");

    let _ = bindtextdomain(gettext_package, &locale_dir);
    let _ = bind_textdomain_codeset(gettext_package, "UTF-8");

    let lib_domains = [
        "libadwaita",
        "libadwaita-1",
        "adw",
        "adwaita",
        "gtk40",
        "gtk",
    ];

    let lib_locale_dir = if locale_dir == "/usr/share/locale"
        && std::path::Path::new("/usr/share/khushu/locale").exists()
    {
        "/usr/share/khushu/locale".to_string()
    } else if locale_dir == "/app/share/locale"
        && std::path::Path::new("/app/share/khushu/locale").exists()
    {
        "/app/share/khushu/locale".to_string()
    } else if let Ok(snap) = std::env::var("SNAP") {
        let snap_lib_locale = format!("{}/usr/share/khushu/locale", snap);
        if std::path::Path::new(&snap_lib_locale).exists() {
            snap_lib_locale
        } else {
            locale_dir.clone()
        }
    } else {
        locale_dir.clone()
    };

    for domain in lib_domains {
        let _ = bindtextdomain(domain, &lib_locale_dir);
        let _ = bind_textdomain_codeset(domain, "UTF-8");
    }

    crate::background::update_tray_labels(lang);
}

pub fn rebind_locale_after_adw_init() {
    let locale_dir = get_locale_dir();
    let gettext_package = option_env!("GETTEXT_PACKAGE").unwrap_or("khushu");

    let _ = bindtextdomain(gettext_package, &locale_dir);
    let _ = bind_textdomain_codeset(gettext_package, "UTF-8");

    let lib_domains = [
        "libadwaita",
        "libadwaita-1",
        "adw",
        "adwaita",
        "gtk40",
        "gtk",
    ];

    let lib_locale_dir = if locale_dir == "/usr/share/locale"
        && std::path::Path::new("/usr/share/khushu/locale").exists()
    {
        "/usr/share/khushu/locale".to_string()
    } else if locale_dir == "/app/share/locale"
        && std::path::Path::new("/app/share/khushu/locale").exists()
    {
        "/app/share/khushu/locale".to_string()
    } else if let Ok(snap) = std::env::var("SNAP") {
        let snap_lib_locale = format!("{}/usr/share/khushu/locale", snap);
        if std::path::Path::new(&snap_lib_locale).exists() {
            snap_lib_locale
        } else {
            locale_dir.clone()
        }
    } else {
        locale_dir.clone()
    };

    for domain in lib_domains {
        let _ = bindtextdomain(domain, &lib_locale_dir);
        let _ = bind_textdomain_codeset(domain, "UTF-8");
    }
}

pub fn tr(key: &str, _lang: &str) -> String {
    if key == "translator-credits" {
        let res = dgettext("khushu", key);
        if res != key && !res.is_empty() {
            return res;
        }
        return "Djalel Oukid".to_string();
    }

    let res = dgettext("khushu", key);
    if res != key && !res.is_empty() {
        return res;
    }

    let fallback_res = dgettext("khushu-gtk", key);
    if fallback_res != key && !fallback_res.is_empty() {
        return fallback_res;
    }

    let adw_res = dgettext("libadwaita", key);
    if adw_res != key && !adw_res.is_empty() {
        return adw_res;
    }

    key.to_string()
}
