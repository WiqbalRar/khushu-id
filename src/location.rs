use crate::i18n::tr;
use reqwest::Client;
use serde::Deserialize;
use std::sync::OnceLock;

static HTTP_CLIENT: OnceLock<Client> = OnceLock::new();

fn client() -> &'static Client {
    HTTP_CLIENT.get_or_init(|| {
        Client::builder()
            .user_agent("Khushu-Prayer-App/1.0.0")
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .expect("Failed to create HTTP client")
    })
}

#[derive(Deserialize, Debug)]
pub struct GeocodeResult {
    pub lat: String,
    pub lon: String,
    pub display_name: String,
}

pub fn short_city_with_country(display_name: &str) -> String {
    let parts: Vec<&str> = display_name
        .split(',')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect();
    if parts.len() >= 2 {
        format!("{}, {}", parts[0], parts[parts.len() - 1])
    } else if let Some(first) = parts.first() {
        first.to_string()
    } else {
        display_name.to_string()
    }
}

use ashpd::desktop::location::{Accuracy, LocationProxy};
use futures_util::StreamExt;

pub async fn fetch_auto_location(lang: &str) -> Result<(f64, f64, String), String> {
    fetch_geoclue_location(lang).await
}

async fn fetch_geoclue_location(lang: &str) -> Result<(f64, f64, String), String> {
    log::info!("Attempting to fetch location via ASHPD Portal...");

    let proxy = LocationProxy::new().await.map_err(|e| {
        log::error!("Failed to create Location proxy: {}", e);
        tr(
            "Location service unavailable. Please check system settings.",
            lang,
        )
    })?;

    let session = proxy
        .create_session(None, None, Some(Accuracy::City))
        .await
        .map_err(|e| {
            log::error!("Failed to create location session: {}", e);
            tr("Location access denied or unavailable.", lang)
        })?;

    let mut stream = proxy.receive_location_updated().await.map_err(|e| {
        log::error!("Failed to receive location updates: {}", e);
        tr("Failed to receive location updates.", lang)
    })?;

    proxy.start(&session, None).await.map_err(|e| {
        log::error!("Failed to start location session: {}", e);
        tr("Location access denied or unavailable.", lang)
    })?;

    let location_result =
        tokio::time::timeout(std::time::Duration::from_secs(10), stream.next()).await;

    let location = match location_result {
        Ok(Some(loc)) => loc,
        Ok(None) => {
            let _ = session.close().await;
            log::error!("Location stream ended unexpectedly");
            return Err(tr("Location service disconnected unexpectedly.", lang));
        }
        Err(_) => {
            let _ = session.close().await;
            log::error!("Location request timed out (possible permission denial)");
            return Err(tr(
                "Location request timed out. Please check your system settings.",
                lang,
            ));
        }
    };

    let lat = location.latitude();
    let lon = location.longitude();

    let _ = session.close().await;

    log::info!("Portal location fetched: {}, {}", lat, lon);

    let city = match reverse_geocode(lat, lon, lang).await {
        Ok(name) => {
            log::info!("Reverse geocoded to: {}", name);
            short_city_with_country(&name)
        }
        Err(e) => {
            log::warn!("Reverse geocode failed, using coordinates: {}", e);
            format_coordinates(lat, lon)
        }
    };

    Ok((lat, lon, city))
}

async fn reverse_geocode(lat: f64, lon: f64, lang: &str) -> Result<String, String> {
    let http = client();

    let url = format!(
        "https://nominatim.openstreetmap.org/reverse?lat={}&lon={}&format=json&zoom=10&accept-language={}",
        lat, lon, lang
    );

    let resp = http
        .get(url)
        .send()
        .await
        .map_err(|_| tr("Network error while resolving city.", lang))?;

    let result: GeocodeResult = resp
        .json()
        .await
        .map_err(|_| tr("Invalid response from location service.", lang))?;

    if result.display_name.is_empty() {
        return Err(tr("Could not find city name for these coordinates.", lang));
    }

    Ok(result.display_name)
}

fn format_coordinates(lat: f64, lon: f64) -> String {
    let lat_dir = if lat >= 0.0 { "N" } else { "S" };
    let lon_dir = if lon >= 0.0 { "E" } else { "W" };
    format!("{:.2}°{}, {:.2}°{}", lat.abs(), lat_dir, lon.abs(), lon_dir)
}

pub async fn search_city(query: &str, lang: &str) -> Result<(f64, f64, String), String> {
    log::info!("Searching for city: {}", query);
    let http = client();

    let url = format!(
        "https://nominatim.openstreetmap.org/search?q={}&format=json&limit=1&accept-language={}",
        urlencoding::encode(query),
        lang
    );

    let resp = http.get(url).send().await.map_err(|e| {
        log::error!("Geocoding request failed: {}", e);
        tr("Network error. Please check your connection.", lang)
    })?;

    let results: Vec<GeocodeResult> = resp.json().await.map_err(|e| {
        log::error!("Geocoding JSON parsing failed: {}", e);
        tr("Invalid response from location service.", lang)
    })?;

    if let Some(res) = results.first() {
        let lat = res.lat.parse::<f64>().map_err(|_| {
            log::error!("Invalid latitude from API: {}", res.lat);
            tr("Invalid response from location service.", lang)
        })?;
        let lon = res.lon.parse::<f64>().map_err(|_| {
            log::error!("Invalid longitude from API: {}", res.lon);
            tr("Invalid response from location service.", lang)
        })?;
        log::info!("City found: {} ({}, {})", res.display_name, lat, lon);
        Ok((lat, lon, res.display_name.clone()))
    } else {
        log::warn!("City not found for query: {}", query);
        Err(tr("City not found. Please check the spelling.", lang))
    }
}
