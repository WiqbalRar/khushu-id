# Changelog

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
