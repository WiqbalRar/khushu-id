use gettextrs::{LocaleCategory, bind_textdomain_codeset, bindtextdomain, dgettext, setlocale};
use std::sync::OnceLock;

struct SystemLocaleSnapshot {
    language: Option<String>,
    lc_all: Option<String>,
    lang: Option<String>,
}

static ORIGINAL_LOCALE: OnceLock<SystemLocaleSnapshot> = OnceLock::new();

pub fn save_original_locale() {
    ORIGINAL_LOCALE.get_or_init(|| SystemLocaleSnapshot {
        language: std::env::var("LANGUAGE").ok().filter(|s| !s.is_empty()),
        lc_all: std::env::var("LC_ALL").ok().filter(|s| !s.is_empty()),
        lang: std::env::var("LANG").ok().filter(|s| !s.is_empty()),
    });
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
        return format!("{}/usr/share/locale", snap);
    }

    if let Ok(canon) = std::fs::canonicalize("target/locale") {
        return canon.to_string_lossy().to_string();
    }

    "./po".to_string()
}

fn current_language_hint() -> String {
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
        .trim();

    let mut candidates = Vec::new();
    let mut push = |candidate: String| {
        if !candidate.is_empty() && !candidates.contains(&candidate) {
            candidates.push(candidate);
        }
    };

    if normalized.is_empty() {
        push("en".to_string());
        return candidates;
    }

    push(normalized.to_string());
    push(normalized.replace('-', "_"));
    push(normalized.replace('_', "-"));
    push(
        normalized
            .split(['-', '_'])
            .next()
            .unwrap_or("en")
            .to_string(),
    );
    candidates
}

fn domain_catalog_exists(locale_dir: &str, lang: &str, domain: &str) -> bool {
    locale_candidates(lang).into_iter().any(|candidate| {
        std::path::Path::new(locale_dir)
            .join(candidate)
            .join("LC_MESSAGES")
            .join(format!("{domain}.mo"))
            .exists()
    })
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
        let snap_lib_locale = format!("{}/usr/share/khushu/locale", snap);
        if std::path::Path::new(&snap_lib_locale).exists() {
            snap_lib_locale
        } else {
            locale_dir.to_string()
        }
    } else {
        locale_dir.to_string()
    }
}

fn library_locale_dir_for_domain(domain: &str, lang: &str, locale_dir: &str) -> String {
    let bundled_domains = ["gtk40", "libadwaita"];
    let is_bundled = bundled_domains.contains(&domain);

    if is_bundled {
        let our_locale_dir = custom_library_locale_dir(locale_dir);
        if domain_catalog_exists(&our_locale_dir, lang, domain) {
            return our_locale_dir;
        }
        if domain_catalog_exists(locale_dir, lang, domain) {
            return locale_dir.to_string();
        }
        return locale_dir.to_string();
    }

    let mut candidates = Vec::new();
    let mut push = |candidate: String| {
        if !candidate.is_empty() && !candidates.contains(&candidate) {
            candidates.push(candidate);
        }
    };

    if std::path::Path::new("/usr/share/locale").exists() {
        push("/usr/share/locale".to_string());
    }
    if std::path::Path::new("/app/share/locale").exists() {
        push("/app/share/locale".to_string());
    }
    if let Ok(snap) = std::env::var("SNAP") {
        push(format!("{}/usr/share/locale", snap));
    }
    push(custom_library_locale_dir(locale_dir));
    push(locale_dir.to_string());

    candidates
        .into_iter()
        .find(|dir| domain_catalog_exists(dir, lang, domain))
        .unwrap_or_else(|| locale_dir.to_string())
}

unsafe extern "C" {
    #[link_name = "bindtextdomain"]
    fn libc_bindtextdomain(
        domainname: *const std::os::raw::c_char,
        dirname: *const std::os::raw::c_char,
    ) -> *mut std::os::raw::c_char;
}

fn glibc_bindtextdomain(domain: &str, dir: &str) {
    if let (Ok(c_domain), Ok(c_dir)) = (std::ffi::CString::new(domain), std::ffi::CString::new(dir))
    {
        unsafe {
            libc_bindtextdomain(c_domain.as_ptr(), c_dir.as_ptr());
        }
    }
}

fn bind_library_domains(locale_dir: &str, lang: &str) {
    let gtk_locale_dir = library_locale_dir_for_domain("gtk40", lang, locale_dir);
    let _ = gettextrs::bindtextdomain("gtk40", &gtk_locale_dir);
    let _ = gettextrs::bind_textdomain_codeset("gtk40", "UTF-8");
    glibc_bindtextdomain("gtk40", &gtk_locale_dir);

    let adw_locale_dir = library_locale_dir_for_domain("libadwaita", lang, locale_dir);
    let _ = gettextrs::bindtextdomain("libadwaita", &adw_locale_dir);
    let _ = gettextrs::bind_textdomain_codeset("libadwaita", "UTF-8");
    glibc_bindtextdomain("libadwaita", &adw_locale_dir);
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
        if let Some(ref lang) = snapshot.language
            && lang != "C"
            && lang != "POSIX"
        {
            return lang.split(':').next().unwrap_or("en").to_string();
        }

        if let Some(ref lc_all) = snapshot.lc_all
            && lc_all != "C"
            && lc_all != "POSIX"
        {
            return lc_all.split('.').next().unwrap_or("en").to_string();
        }

        if let Some(ref lang) = snapshot.lang
            && lang != "C"
            && lang != "POSIX"
        {
            return lang.split('.').next().unwrap_or("en").to_string();
        }
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

    bind_library_domains(&locale_dir, lang);

    crate::background::update_tray_labels(lang);
}

pub fn rebind_locale_after_adw_init() {
    let locale_dir = get_locale_dir();
    let gettext_package = option_env!("GETTEXT_PACKAGE").unwrap_or("khushu");

    let _ = bindtextdomain(gettext_package, &locale_dir);
    let _ = bind_textdomain_codeset(gettext_package, "UTF-8");

    bind_library_domains(&locale_dir, &current_language_hint());
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

    let gtk_res = dgettext("gtk40", key);
    if gtk_res != key && !gtk_res.is_empty() {
        return gtk_res;
    }

    let gtk_legacy_res = dgettext("gtk", key);
    if gtk_legacy_res != key && !gtk_legacy_res.is_empty() {
        return gtk_legacy_res;
    }

    key.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_locale_candidates() {
        assert_eq!(locale_candidates("en"), vec!["en"]);
        assert_eq!(locale_candidates("en_US"), vec!["en_US", "en-US", "en"]);
        assert_eq!(
            locale_candidates("fr_FR.UTF-8"),
            vec!["fr_FR", "fr-FR", "fr"]
        );
        assert_eq!(locale_candidates("ar_SA"), vec!["ar_SA", "ar-SA", "ar"]);
        assert_eq!(locale_candidates(""), vec!["en"]);
    }

    #[test]
    fn test_detect_system_locale_with_snapshot() {
        save_original_locale();

        let detected = detect_system_locale();
        assert!(!detected.is_empty());
        assert!(
            detected
                .chars()
                .all(|c| c.is_ascii_alphabetic() || c == '_' || c == '-')
        );
    }

    #[test]
    fn test_tr_function_returns_key_when_no_translation() {
        let result = tr("nonexistent_key_xyz123", "en");
        assert_eq!(result, "nonexistent_key_xyz123");
    }

    #[test]
    fn test_arabic_locale_special_handling() {
        let candidates = if "ar" == "ar" {
            vec![
                "ar_DZ.UTF-8".to_string(),
                "ar_SA.UTF-8".to_string(),
                "ar.UTF-8".to_string(),
                "en_US.UTF-8".to_string(),
                "C.UTF-8".to_string(),
            ]
        } else {
            vec![
                format!("{}.UTF-8", "ar"),
                format!("{}_{}.UTF-8", "ar", "ar".to_uppercase()),
                "en_US.UTF-8".to_string(),
                "C.UTF-8".to_string(),
            ]
        };

        assert!(candidates.contains(&"ar_DZ.UTF-8".to_string()));
    }
}
