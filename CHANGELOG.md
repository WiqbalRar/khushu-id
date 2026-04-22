# Changelog

## [1.1.0] — 2026-04-22

### Added
- Added the Noble Quran module — a fully offline reader featuring all 114 surahs with Uthmanic Arabic script (Amiri Quran font), parallel translations in five languages, persistent bookmarks, reading-position memory, full-text search with Arabic diacritics normalization, mushaf-style page navigation, and user-adjustable Arabic/translation font sizes and line spacing
- Added ICU-backed localization for city, country and timezone labels
- Added validated custom IANA timezone editing with case-insensitive normalization
- Added Mawaqit API integration as an alternative prayer times source
- Added a stop Adhan button on the main page for quick access when notifications are missed
- Added Iqamah countdown timer on the prayer times page with per-prayer delay controls in Settings
- Added comprehensive test suite verifying Arabic language support including Amiri font application, RTL direction, and tray label translations

### Improved
- Reorganized Settings page into General, Prayer Setup, and Notifications & Audio sections
- Improved Qibla compass lifecycle handling to prevent excessive resource consumption
- Improved UI update logic for both manual and automatic language changes with better system locale detection

### Fixed
- Fixed Snap tray icon visibility via an [upstream patch to ksni](https://github.com/iovxw/ksni/pull/37) (resolves AppArmor D-Bus blocking).

## [1.0.3] — 2026-03-30

### Fixed
- Fixed Snap autostart and notification icon rendering
- Improved locale detection and fallback logic

## [1.0.2] — 2026-03-24

### Fixed
- Fixed critical autostart bug where GNOME Background Portal would dynamically delete autostart entries.
- Fixed Flatpak autostart command to launch silently in the background instead of popping open the main UI.
- Fixed translation extraction pipeline for proper nouns (e.g., Muslim, At-Tirmidhi) to prevent unintended fuzzy matching across languages.

### Added
- Expanded AppStream metadata coverage with 3 additional screenshots.
- Added comprehensive translations for all AppStream screenshot captions across all supported languages.

## [1.0.1] — 2026-03-18

### Fixed
- Fixed tray icon translation extraction (Quit and Open Khushu)
- Fixed AppStream metadata translation (descriptions, keywords)
- Improved tray behavior to update labels immediately on language change

## [1.0.0] — 2026-03-04

### Added
- Accurate prayer times with 11 calculation methods (MWL, ISNA, Egypt, Makkah, Karachi, Dubai, Kuwait, Qatar, Singapore, Turkey, Moonsighting Committee)
- Adhan audio playback with volume control and mute toggle
- Pre-prayer notifications with configurable lead time
- Staggered Adkar notifications (morning, evening, post-prayer)
- Hijri calendar with adjustable offset
- Qibla compass direction
- Automatic location detection via GeoClue
- Manual city search
- System tray icon with quick actions
- Background mode with autostart support
- Multi-language: Arabic, English, French, Spanish, Turkish
- Dark, Light, and System theme modes
- Obfuscated coordinate storage for privacy
- Zero telemetry
