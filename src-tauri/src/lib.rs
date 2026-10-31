mod api_server;
mod error;
mod geocoding;
mod http_cache;
mod routes;
mod tile_protocol;
mod tile_storage;

pub use error::AppError;
pub use geocoding::{dequantize, forward_geocode, reverse_geocode, GeocodingData};
pub use http_cache::{generate_cache_key, validate_method, validate_url, ProxyRequest};
pub use routes::{
    get_all_route_paths_by_mode, get_dataset_stats, get_nearby_stops_with_routes, get_route_path,
    get_routes_for_stop, get_stops_for_route_short_name, RouteMode, RoutesData,
};

use std::sync::Arc;

use http_cache::Cache;
use serde::Serialize;
use tile_protocol::TileProtocolHandler;
use tile_storage::TileCache;

#[derive(Debug, Serialize)]
struct BuildInfo {
    app_version: String,
    dataset_version: String,
    dataset_start_date: String,
    dataset_end_date: String,
    dataset_publisher: String,
    dataset_source_path: String,
}

#[cfg(not(tarpaulin_include))]
#[tauri::command]
fn get_build_info() -> BuildInfo {
    BuildInfo {
        app_version: env!("CARGO_PKG_VERSION").to_string(),
        dataset_version: option_env!("HSL_DATASET_VERSION")
            .unwrap_or_default()
            .to_string(),
        dataset_start_date: option_env!("HSL_DATASET_START_DATE")
            .unwrap_or_default()
            .to_string(),
        dataset_end_date: option_env!("HSL_DATASET_END_DATE")
            .unwrap_or_default()
            .to_string(),
        dataset_publisher: option_env!("HSL_DATASET_PUBLISHER")
            .unwrap_or_default()
            .to_string(),
        dataset_source_path: option_env!("HSL_DATASET_SOURCE_PATH")
            .unwrap_or_default()
            .to_string(),
    }
}

#[cfg(not(tarpaulin_include))]
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() -> Result<(), AppError> {
    // Initialize cache with 5 minute TTL
    let cache = Cache::new(300);

    // Initialize tile cache - use local directory for simplicity
    let tile_cache_dir = std::env::current_dir()?.join(".cache").join("tile_cache");

    // Create cache directory if it doesn't exist
    std::fs::create_dir_all(&tile_cache_dir)?;

    let tile_cache = Arc::new(TileCache::new(&tile_cache_dir)?);
    let tile_cache_for_protocol = Arc::clone(&tile_cache);

    // Load geocoding data
    let geocoding_data = Arc::new(GeocodingData::load()?);
    // Load transit routes data
    let routes_data = Arc::new(RoutesData::load()?);

    // Clone Arcs for the axum API server
    let geocoding_for_api = Arc::clone(&geocoding_data);
    let routes_for_api = Arc::clone(&routes_data);

    tauri::Builder::default()
        .plugin(
            tauri_plugin_log::Builder::default()
                .level(log::LevelFilter::Info)
                .build(),
        )
        .manage(cache)
        .manage(tile_cache)
        .manage(geocoding_data)
        .manage(routes_data)
        .invoke_handler(tauri::generate_handler![
            http_cache::fetch_with_proxy,
            http_cache::clear_cache,
            http_cache::get_cache_stats,
            get_build_info,
            tile_protocol::clear_tile_cache,
            tile_protocol::get_tile_cache_stats,
            reverse_geocode,
            forward_geocode,
            get_route_path,
            get_routes_for_stop,
            get_nearby_stops_with_routes,
            get_stops_for_route_short_name,
            get_all_route_paths_by_mode,
            get_dataset_stats
        ])
        .register_asynchronous_uri_scheme_protocol(
            "tile",
            move |_app_handle, request, responder| {
                let tile_cache = Arc::clone(&tile_cache_for_protocol);

                tauri::async_runtime::spawn(async move {
                    let handler = TileProtocolHandler::new(tile_cache);
                    let response = handler.handle_request(&request).await;
                    responder.respond(response);
                });
            },
        )
        .setup(move |_app| {
            log::info!("Tile cache initialized and protocol registered");

            // Spawn the axum REST API server for browser-accessible endpoints
            tauri::async_runtime::spawn(api_server::start_api_server(
                geocoding_for_api,
                routes_for_api,
            ));

            Ok(())
        })
        .run(tauri::generate_context!())
        .map_err(|e| AppError::Http(format!("Failed to run Tauri application: {}", e)))?;

    Ok(())
}
