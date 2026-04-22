//! This module provides helper functions for setting up test environments,

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Once;

static INIT: Once = Once::new();

pub fn setup_test_env() {
    INIT.call_once(|| {
        let _ = fs::create_dir_all("target/test-resources");
        let _ = env_logger::builder()
            .filter_level(log::LevelFilter::Warn)
            .is_test(true)
            .try_init();
    });
}

pub fn create_test_config(content: &str) -> TempConfig {
    setup_test_env();

    let config_dir = PathBuf::from("target/test-resources/config");
    fs::create_dir_all(&config_dir).expect("Failed to create test config directory");

    let config_path = config_dir.join(format!("test-config-{}.json", std::process::id()));

    fs::write(&config_path, content).expect("Failed to write test config");

    TempConfig { path: config_path }
}

pub struct TempConfig {
    path: PathBuf,
}

impl TempConfig {
    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TempConfig {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

pub fn load_test_resource(relative_path: &str) -> String {
    setup_test_env();

    let project_path = PathBuf::from(relative_path);
    if project_path.exists() {
        return fs::read_to_string(&project_path)
            .unwrap_or_else(|_| panic!("Failed to read test resource: {}", relative_path));
    }

    let test_path = PathBuf::from("target/test-resources").join(relative_path);
    if test_path.exists() {
        return fs::read_to_string(&test_path)
            .unwrap_or_else(|_| panic!("Failed to read test resource: {}", relative_path));
    }

    panic!("Test resource not found: {}", relative_path);
}

pub fn file_contains_patterns(file_path: &Path, patterns: &[&str]) -> bool {
    if !file_path.exists() {
        return false;
    }

    let content = match fs::read_to_string(file_path) {
        Ok(content) => content,
        Err(_) => return false,
    };

    patterns.iter().all(|pattern| content.contains(pattern))
}

pub fn create_test_po_file(lang: &str, translations: &[(&str, &str)]) -> TempFile {
    setup_test_env();

    let mut content = String::new();
    content.push_str(&format!(
        "msgid \"\"\nmsgstr \"\"\n\"Language: {}\\n\"\n\"Content-Type: text/plain; charset=UTF-8\\n\"\n\n",
        lang
    ));

    for (msgid, msgstr) in translations {
        content.push_str(&format!("msgid \"{}\"\nmsgstr \"{}\"\n\n", msgid, msgstr));
    }

    create_temp_file(&format!("test-{}.po", lang), &content)
}

pub fn create_temp_file(filename: &str, content: &str) -> TempFile {
    setup_test_env();

    let test_dir = PathBuf::from("target/test-resources/temp");
    fs::create_dir_all(&test_dir).expect("Failed to create test temp directory");

    let file_path = test_dir.join(filename);
    fs::write(&file_path, content).expect("Failed to write temp file");

    TempFile { path: file_path }
}

pub struct TempFile {
    path: PathBuf,
}

impl TempFile {
    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn content(&self) -> String {
        fs::read_to_string(&self.path).expect("Failed to read temp file")
    }
}

impl Drop for TempFile {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

pub fn assert_file_contains_patterns(file_path: &Path, patterns: &[&str]) {
    assert!(file_path.exists(), "File does not exist: {:?}", file_path);

    let content = fs::read_to_string(file_path).expect("Failed to read file for pattern checking");

    for pattern in patterns {
        assert!(
            content.contains(pattern),
            "File {:?} does not contain pattern: {}\nFile content:\n{}",
            file_path,
            pattern,
            content
        );
    }
}

pub fn assert_arabic_support_patterns(file_path: &Path) {
    let arabic_patterns = ["Amiri", "font-family", "arabic-text", "quran-arabic"];

    assert_file_contains_patterns(file_path, &arabic_patterns);
}
