use std::env;
use std::fs;
use std::path::Path;
use std::process::Command;

fn has_tool(name: &str) -> bool {
    Command::new(name)
        .arg("--version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok_and(|s| s.success())
}

fn run_cmd(cmd: &mut Command, context: &str) -> bool {
    match cmd.status() {
        Ok(status) if status.success() => true,
        Ok(status) => {
            println!(
                "cargo:warning={context}: exited with {}",
                status
                    .code()
                    .map_or("signal".to_string(), |c| c.to_string())
            );
            false
        }
        Err(e) => {
            println!("cargo:warning={context}: failed to execute: {e}");
            false
        }
    }
}

fn try_merge_with_system(
    our_po: &Path,
    system_mo: &str,
    mo_path: &Path,
    tmp_dir: &Path,
    domain: &str,
    lang: &str,
) -> bool {
    let tmp_sys_po = tmp_dir.join(format!("{}_sys_{}.po", domain, lang));
    let tmp_merged_po = tmp_dir.join(format!("{}_merged_{}.po", domain, lang));

    let cleanup = || {
        let _ = fs::remove_file(&tmp_sys_po);
        let _ = fs::remove_file(&tmp_merged_po);
    };

    if !run_cmd(
        Command::new("msgunfmt")
            .arg(system_mo)
            .arg("-o")
            .arg(&tmp_sys_po),
        &format!("msgunfmt {domain}/{lang}"),
    ) {
        cleanup();
        return false;
    }

    if !run_cmd(
        Command::new("msgcat")
            .arg("--use-first")
            .arg(our_po)
            .arg(&tmp_sys_po)
            .arg("-o")
            .arg(&tmp_merged_po),
        &format!("msgcat merge {domain}/{lang}"),
    ) {
        cleanup();
        return false;
    }

    let ok = run_cmd(
        Command::new("msgfmt")
            .arg("-o")
            .arg(mo_path)
            .arg(&tmp_merged_po),
        &format!("msgfmt merged {domain}/{lang}"),
    );

    cleanup();
    ok
}

fn main() {
    println!("cargo:rerun-if-changed=data/");
    println!("cargo:rerun-if-changed=po/");

    if !Path::new("data/khushu.gresource.xml").exists() {
        panic!(
            "CRITICAL: data/khushu.gresource.xml not found! Current dir: {:?}",
            env::current_dir()
        );
    }

    glib_build_tools::compile_resources(
        &["data"],
        "data/khushu.gresource.xml",
        "khushu-resources.gresource",
    );

    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let locale_dir = Path::new(&manifest_dir).join("target").join("locale");

    let bundled_domains = ["gtk40", "libadwaita"];

    let can_merge = has_tool("msgunfmt") && has_tool("msgcat");
    if !can_merge {
        println!(
            "cargo:warning=msgunfmt/msgcat not found; \
             bundled domains will not be merged with system catalogs"
        );
    }

    let system_locale_dirs: Vec<String> = [
        Some("/usr/share/locale".to_string()),
        Some("/app/share/locale".to_string()),
        env::var("SNAP")
            .ok()
            .map(|s| format!("{}/usr/share/locale", s)),
    ]
    .into_iter()
    .flatten()
    .collect();

    if !has_tool("msgfmt") {
        println!("cargo:warning=msgfmt not found. Is gettext installed? Skipping .po compilation.");
        return;
    }

    if let Ok(entries) = fs::read_dir("po") {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("po") {
                continue;
            }
            let file_stem = path.file_stem().unwrap().to_str().unwrap();

            let parts: Vec<&str> = file_stem.split('.').collect();
            let (domain, lang) = if parts.len() == 1 {
                ("khushu", parts[0])
            } else {
                (parts[0], parts[1])
            };

            if lang == "pot" || file_stem.ends_with(".pot") {
                continue;
            }

            let lang_dir = locale_dir.join(lang).join("LC_MESSAGES");
            fs::create_dir_all(&lang_dir).unwrap();

            let mo_path = lang_dir.join(format!("{}.mo", domain));

            let is_bundled = bundled_domains.contains(&domain);
            if is_bundled && can_merge {
                let system_mo = system_locale_dirs
                    .iter()
                    .map(|dir| format!("{}/{}/LC_MESSAGES/{}.mo", dir, lang, domain))
                    .find(|p| Path::new(p).exists());

                if let Some(sys_mo) = system_mo {
                    if try_merge_with_system(&path, &sys_mo, &mo_path, &locale_dir, domain, lang) {
                        continue;
                    }
                    println!(
                        "cargo:warning=i18n: merge failed for {domain}/{lang}, \
                         falling back to our .po only"
                    );
                }
            }

            run_cmd(
                Command::new("msgfmt").arg("-o").arg(&mo_path).arg(&path),
                &format!("msgfmt {domain}/{lang}"),
            );
        }
    }
}
