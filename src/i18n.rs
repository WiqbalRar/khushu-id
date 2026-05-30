use gettextrs::{LocaleCategory, bind_textdomain_codeset, bindtextdomain, dgettext, setlocale};
use std::sync::{OnceLock, RwLock};

static ORIGINAL_LOCALE: OnceLock<SystemLocaleSnapshot> = OnceLock::new();
static CURRENT_APP_LOCALE: OnceLock<RwLock<String>> = OnceLock::new();

struct SystemLocaleSnapshot {
    language: Option<String>,
    lc_all: Option<String>,
    lang: Option<String>,
}

pub fn save_original_locale() {
    ORIGINAL_LOCALE.get_or_init(|| SystemLocaleSnapshot {
        language: std::env::var("LANGUAGE").ok().filter(|s| !s.is_empty()),
        lc_all: std::env::var("LC_ALL").ok().filter(|s| !s.is_empty()),
        lang: std::env::var("LANG").ok().filter(|s| !s.is_empty()),
    });
}

fn locale_fallback_order(lang: &str) -> Vec<String> {
    match lang {
        "ar" => vec![
            "ar_DZ.UTF-8".to_string(),
            "ar_SA.UTF-8".to_string(),
            "ar.UTF-8".to_string(),
            "en_US.UTF-8".to_string(),
            "C.UTF-8".to_string(),
        ],
        "fr" => vec![
            "fr_FR.UTF-8".to_string(),
            "fr.UTF-8".to_string(),
            "en_US.UTF-8".to_string(),
            "C.UTF-8".to_string(),
        ],
        "es" => vec![
            "es_ES.UTF-8".to_string(),
            "es.UTF-8".to_string(),
            "en_US.UTF-8".to_string(),
            "C.UTF-8".to_string(),
        ],
        "tr" => vec![
            "tr_TR.UTF-8".to_string(),
            "tr.UTF-8".to_string(),
            "en_US.UTF-8".to_string(),
            "C.UTF-8".to_string(),
        ],
        "id" => vec![
            "id_ID.UTF-8".to_string(),
            "id.UTF-8".to_string(),
            "en_US.UTF-8".to_string(),
            "C.UTF-8".to_string(),
        ],
        _ => vec![
            format!("{}.UTF-8", lang),
            "en_US.UTF-8".to_string(),
            "C.UTF-8".to_string(),
        ],
    }
}

fn apply_locale(lang: &str) {
    if lang == "auto" || lang.is_empty() {
        setlocale(LocaleCategory::LcAll, "");
        return;
    }

    for loc in locale_fallback_order(lang) {
        if setlocale(LocaleCategory::LcAll, &*loc).is_some() {
            return;
        }
    }
    setlocale(LocaleCategory::LcAll, b"C.UTF-8" as &[u8]);
}

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
        let app_locale = format!("{}/usr/share/khushu/locale", snap);
        if std::path::Path::new(&app_locale).exists() {
            return app_locale;
        }
        return format!("{}/usr/share/locale", snap);
    }

    if let Ok(canon) = std::fs::canonicalize("target/locale") {
        return canon.to_string_lossy().to_string();
    }

    if std::path::Path::new("po").exists() {
        return "po".to_string();
    }

    "./po".to_string()
}

fn current_language_hint() -> String {
    if let Some(lock) = CURRENT_APP_LOCALE.get()
        && let Ok(current) = lock.read()
        && !current.is_empty()
        && current.as_str() != "auto"
    {
        return current.clone();
    }

    std::env::var("LANGUAGE")
        .ok()
        .filter(|s| !s.is_empty())
        .or_else(|| std::env::var("LC_ALL").ok().filter(|s| !s.is_empty()))
        .or_else(|| std::env::var("LANG").ok().filter(|s| !s.is_empty()))
        .unwrap_or_else(|| "en".to_string())
}

fn locale_candidates(lang: &str) -> Vec<String> {
    let normalized = lang
        .split(':')
        .next()
        .unwrap_or_default()
        .split('@')
        .next()
        .unwrap_or_default()
        .split('.')
        .next()
        .unwrap_or_default()
        .trim()
        .to_string();

    if normalized.is_empty() {
        return vec!["en".to_string()];
    }

    let mut candidates = vec![normalized.clone()];
    if normalized.contains('_') {
        candidates.push(normalized.replace('_', "-"));
    } else if normalized.contains('-') {
        candidates.push(normalized.replace('-', "_"));
    }
    if let Some(base) = normalized.split(['-', '_']).next()
        && !candidates.contains(&base.to_string())
    {
        candidates.push(base.to_string());
    }
    candidates
}

fn domain_catalog_exists(dir: &str, lang: &str, domain: &str) -> bool {
    for candidate in locale_candidates(lang) {
        let mo_path = format!("{}/{}/LC_MESSAGES/{}.mo", dir, candidate, domain);
        if std::path::Path::new(&mo_path).exists() {
            return true;
        }
    }
    false
}

fn custom_library_locale_dir(locale_dir: &str) -> String {
    if locale_dir == "/usr/share/locale"
        && std::path::Path::new("/usr/share/khushu/locale").exists()
    {
        "/usr/share/khushu/locale".to_string()
    } else if locale_dir == "/app/share/locale"
        && std::path::Path::new("/app/share/khushu/locale").exists()
    {
        "/app/share/khushu/locale".to_string()
    } else if let Ok(snap) = std::env::var("SNAP") {
        let snap_locale = format!("{}/usr/share/khushu/locale", snap);
        if std::path::Path::new(&snap_locale).exists() {
            return snap_locale;
        }
        locale_dir.to_string()
    } else {
        locale_dir.to_string()
    }
}

fn library_locale_dir_for_domain(domain: &str, lang: &str, locale_dir: &str) -> String {
    let bundled = ["gtk40", "libadwaita"];
    if bundled.contains(&domain) {
        let our_dir = custom_library_locale_dir(locale_dir);
        if domain_catalog_exists(&our_dir, lang, domain) {
            return our_dir;
        }
    }

    let mut candidates = vec![
        "/usr/share/locale".to_string(),
        "/usr/share/locale-langpack".to_string(),
    ];

    if let Ok(snap) = std::env::var("SNAP") {
        candidates.push(format!("{}/usr/share/locale", snap));
    }
    candidates.push(custom_library_locale_dir(locale_dir));
    candidates.push(locale_dir.to_string());

    candidates
        .into_iter()
        .find(|d| domain_catalog_exists(d, lang, domain))
        .unwrap_or_else(|| locale_dir.to_string())
}

fn bind_library_domains(locale_dir: &str, lang: &str) {
    let gtk_dir = library_locale_dir_for_domain("gtk40", lang, locale_dir);
    let _ = bindtextdomain("gtk40", &gtk_dir);
    let _ = bind_textdomain_codeset("gtk40", "UTF-8");

    let adw_dir = library_locale_dir_for_domain("libadwaita", lang, locale_dir);
    let _ = bindtextdomain("libadwaita", &adw_dir);
    let _ = bind_textdomain_codeset("libadwaita", "UTF-8");
}

pub fn update_locale(lang: &str) {
    if lang == "auto" || lang.is_empty() {
        let system_lang = detect_system_locale();
        update_locale_internal(&system_lang);
    } else {
        update_locale_internal(lang);
    }
}

pub fn detect_system_locale() -> String {
    if let Some(snapshot) = ORIGINAL_LOCALE.get() {
        if let Some(ref l) = snapshot.language
            && l != "C"
            && l != "POSIX"
        {
            return l.split(':').next().unwrap_or("en").to_string();
        }
        if let Some(ref l) = snapshot.lc_all
            && l != "C"
            && l != "POSIX"
        {
            return l.split('.').next().unwrap_or("en").to_string();
        }
        if let Some(ref l) = snapshot.lang
            && l != "C"
            && l != "POSIX"
        {
            return l.split('.').next().unwrap_or("en").to_string();
        }
    }

    if let Some(actual) = setlocale(LocaleCategory::LcAll, "") {
        let s = String::from_utf8_lossy(&actual);
        if !s.is_empty() && s != "C" && s != "POSIX" {
            return s.split('_').next().unwrap_or("en").to_string();
        }
    }

    "en".to_string()
}

fn update_locale_internal(lang: &str) {
    CURRENT_APP_LOCALE.get_or_init(|| RwLock::new(lang.to_string()));

    apply_locale(lang);

    let locale_dir = get_locale_dir();
    let pkg = option_env!("GETTEXT_PACKAGE").unwrap_or("khushu");

    let _ = bindtextdomain(pkg, &locale_dir);
    let _ = bind_textdomain_codeset(pkg, "UTF-8");

    bind_library_domains(&locale_dir, lang);

    if let Some(lock) = CURRENT_APP_LOCALE.get()
        && let Ok(mut cur) = lock.write()
    {
        *cur = lang.to_string();
    }

    crate::background::update_tray_labels(lang);
}

pub fn rebind_locale_after_adw_init() {
    let hint = current_language_hint();
    apply_locale(&hint);

    let locale_dir = get_locale_dir();
    let pkg = option_env!("GETTEXT_PACKAGE").unwrap_or("khushu");

    let _ = bindtextdomain(pkg, &locale_dir);
    let _ = bind_textdomain_codeset(pkg, "UTF-8");

    bind_library_domains(&locale_dir, &hint);
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

    let fallback = dgettext("khushu-gtk", key);
    if fallback != key && !fallback.is_empty() {
        return fallback;
    }

    let adw = dgettext("libadwaita", key);
    if adw != key && !adw.is_empty() {
        return adw;
    }

    let gtk = dgettext("gtk40", key);
    if gtk != key && !gtk.is_empty() {
        return gtk;
    }

    let gtk_legacy = dgettext("gtk", key);
    if gtk_legacy != key && !gtk_legacy.is_empty() {
        return gtk_legacy;
    }

    key.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_native_locale_switching() {
        let locale_dir = get_locale_dir();
        assert!(
            std::path::Path::new(&locale_dir).exists(),
            "MO files must exist at {locale_dir} — build with `cargo build` first"
        );

        let _ = bindtextdomain("khushu", &locale_dir);
        let _ = bind_textdomain_codeset("khushu", "UTF-8");

        let r = setlocale(LocaleCategory::LcAll, "fr_FR.UTF-8");
        if r.is_none() {
            eprintln!("Skipping test: fr_FR.UTF-8 locale not available");
            return;
        }

        let fr = dgettext("khushu", "Welcome to Khushu");
        assert_ne!(
            fr, "Welcome to Khushu",
            "dgettext should return French under LC_MESSAGES=fr_FR.UTF-8, got: {fr}"
        );
        println!("fr_FR.UTF-8 → {fr}");

        let r = setlocale(LocaleCategory::LcAll, "ar_SA.UTF-8");
        if r.is_none() {
            eprintln!("Skipping Arabic test: ar_SA.UTF-8 locale not available");
            return;
        }
        let ar = dgettext("khushu", "Welcome to Khushu");
        assert_ne!(
            ar, "Welcome to Khushu",
            "dgettext should return Arabic under LC_MESSAGES=ar_SA.UTF-8, got: {ar}"
        );
        println!("ar_SA.UTF-8 → {ar}");

        assert_ne!(
            fr, ar,
            "French ({fr}) and Arabic ({ar}) must differ — gettext caching bug!"
        );

        setlocale(LocaleCategory::LcAll, "fr_FR.UTF-8");
        let fr2 = dgettext("khushu", "Welcome to Khushu");
        assert_eq!(
            fr, fr2,
            "Switching back to French should return same result: {fr} vs {fr2}"
        );
        println!("fr_FR.UTF-8 (again) → {fr2}");

        println!("NATIVE LOCALE SWITCHING: OK (fr ≠ ar, consistent on round-trip)");
    }
}
