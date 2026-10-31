use crate::AppError;
use converter::TransitDataset;
use serde::Serialize;
use std::collections::HashMap;
use std::sync::Arc;

const FIN_MIN_LAT: f64 = 59.5;
const FIN_MAX_LAT: f64 = 61.0;
const FIN_MIN_LON: f64 = 23.0;
const FIN_MAX_LON: f64 = 26.0;
const EARTH_RADIUS_METERS: f64 = 6_371_000.0;
const ROUTE_PATH_SAMPLE_STEP: usize = 4;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum RouteMode {
    Tram,
    Subway,
    Rail,
    Bus,
    Ferry,
    CableTram,
    AerialLift,
    Funicular,
    Trolleybus,
    Monorail,
    SuburbanRail,
    Coach,
    RegionalBus,
    ExpressBus,
    LocalBus,
    TramService,
    Unknown(u16),
}

impl From<u16> for RouteMode {
    /// Converts a GTFS route_type integer to a [`RouteMode`] variant.
    ///
    /// Unknown type codes are preserved as [`RouteMode::Unknown`].
    ///
    /// # Examples
    ///
    /// ```
    /// use hsl_routes_lib::RouteMode;
    ///
    /// assert_eq!(RouteMode::from(0), RouteMode::Tram);
    /// assert_eq!(RouteMode::from(1), RouteMode::Subway);
    /// assert_eq!(RouteMode::from(2), RouteMode::Rail);
    /// assert_eq!(RouteMode::from(109), RouteMode::SuburbanRail);
    /// assert_eq!(RouteMode::from(999), RouteMode::Unknown(999));
    /// ```
    fn from(value: u16) -> Self {
        match value {
            0 => Self::Tram,
            1 => Self::Subway,
            2 => Self::Rail,
            3 => Self::Bus,
            4 => Self::Ferry,
            5 => Self::CableTram,
            6 => Self::AerialLift,
            7 => Self::Funicular,
            11 => Self::Trolleybus,
            12 => Self::Monorail,
            109 => Self::SuburbanRail,
            700 => Self::Coach,
            701 => Self::RegionalBus,
            702 => Self::ExpressBus,
            704 => Self::LocalBus,
            900 => Self::TramService,
            other => Self::Unknown(other),
        }
    }
}

#[derive(Clone)]
struct RouteSummaryInternal {
    id: u32,
    short_name: String,
    mode_type: RouteMode,
    shapes: Vec<u32>,
}

#[derive(Serialize)]
pub struct Coordinate {
    pub lat: f64,
    pub lon: f64,
}

#[derive(Serialize)]
pub struct RouteSummary {
    pub route_id: u32,
    pub short_name: String,
}

#[derive(Serialize)]
pub struct RoutePath {
    pub route_id: u32,
    pub short_name: String,
    pub shape_paths: Vec<Vec<Coordinate>>,
}

#[derive(Serialize)]
pub struct NearbyStop {
    pub stop_id: u32,
    pub lat: f64,
    pub lon: f64,
    pub distance_meters: f64,
    pub routes: Vec<RouteSummary>,
}

#[derive(Serialize)]
pub struct DatasetStop {
    pub stop_id: u32,
    pub lat: f64,
    pub lon: f64,
}

pub struct RoutesData {
    routes_by_id: HashMap<u32, RouteSummaryInternal>,
    route_ids_by_short_name: HashMap<String, Vec<u32>>,
    route_ids_by_mode: HashMap<RouteMode, Vec<u32>>,
    shapes_by_id: HashMap<u32, Vec<(u16, u16)>>,
    stop_coords: HashMap<u32, (u16, u16)>,
    stop_to_routes: HashMap<u32, Vec<u32>>,
}

impl RoutesData {
    /// Constructs RoutesData from zstd-compressed bitcode bytes (exposed for testing).
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, AppError> {
        let decompressed = zstd::decode_all(bytes)
            .map_err(|e| AppError::Serialization(format!("Failed to decompress data: {}", e)))?;

        let dataset: TransitDataset = bitcode::deserialize(&decompressed)?;
        Ok(Self::from_dataset(dataset))
    }

    /// Constructs RoutesData from a TransitDataset (exposed for testing).
    pub fn from_dataset(dataset: TransitDataset) -> Self {
        let routes_by_id: HashMap<u32, RouteSummaryInternal> = dataset
            .routes
            .into_iter()
            .map(|route| {
                (
                    route.id,
                    RouteSummaryInternal {
                        id: route.id,
                        short_name: route.short_name,
                        mode_type: RouteMode::from(route.route_type),
                        shapes: route.shapes,
                    },
                )
            })
            .collect();

        let mut route_ids_by_short_name: HashMap<String, Vec<u32>> = HashMap::new();
        let mut route_ids_by_mode: HashMap<RouteMode, Vec<u32>> = HashMap::new();
        for route in routes_by_id.values() {
            route_ids_by_short_name
                .entry(route.short_name.clone())
                .or_default()
                .push(route.id);
            route_ids_by_mode
                .entry(route.mode_type)
                .or_default()
                .push(route.id);
        }

        let shapes_by_id = dataset
            .shapes
            .into_iter()
            .map(|shape| (shape.id, shape.points))
            .collect();

        let stop_coords = dataset
            .stops
            .into_iter()
            .map(|(stop_id, lat, lon)| (stop_id, (lat, lon)))
            .collect();

        let stop_to_routes = dataset
            .stop_routes
            .into_iter()
            .map(|entry| (entry.stop_id, entry.routes))
            .collect();

        Self {
            routes_by_id,
            route_ids_by_short_name,
            route_ids_by_mode,
            shapes_by_id,
            stop_coords,
            stop_to_routes,
        }
    }

    pub fn load() -> Result<Self, AppError> {
        todo!("Implement binary loading: read data/routes.bin.zst, decompress with zstd, deserialize with bitcode, call from_dataset()")
    }

    fn route_summary_from_id(&self, route_id: u32) -> Option<RouteSummary> {
        self.routes_by_id.get(&route_id).map(|route| RouteSummary {
            route_id: route.id,
            short_name: route.short_name.clone(),
        })
    }
}

fn dequantize(quantized: u16, min: f64, max: f64) -> f64 {
    let normalized = quantized as f64 / 65535.0;
    min + (normalized * (max - min))
}

fn dequantize_coord(lat: u16, lon: u16) -> Coordinate {
    Coordinate {
        lat: dequantize(lat, FIN_MIN_LAT, FIN_MAX_LAT),
        lon: dequantize(lon, FIN_MIN_LON, FIN_MAX_LON),
    }
}

fn haversine_meters(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f64 {
    let dlat = (lat2 - lat1).to_radians();
    let dlon = (lon2 - lon1).to_radians();
    let lat1_rad = lat1.to_radians();
    let lat2_rad = lat2.to_radians();

    let a =
        (dlat / 2.0).sin().powi(2) + lat1_rad.cos() * lat2_rad.cos() * (dlon / 2.0).sin().powi(2);

    let c = 2.0 * a.sqrt().atan2((1.0 - a).sqrt());
    EARTH_RADIUS_METERS * c
}

fn collect_stop_routes(routes_data: &RoutesData, stop_id: u32) -> Vec<RouteSummary> {
    routes_data
        .stop_to_routes
        .get(&stop_id)
        .into_iter()
        .flat_map(|route_ids| route_ids.iter().copied())
        .filter_map(|route_id| routes_data.route_summary_from_id(route_id))
        .collect()
}

fn longest_shape_points<'a>(
    route: &'a RouteSummaryInternal,
    routes_data: &'a RoutesData,
) -> Option<&'a Vec<(u16, u16)>> {
    route
        .shapes
        .iter()
        .filter_map(|shape_id| routes_data.shapes_by_id.get(shape_id))
        .max_by_key(|points| points.len())
}

pub(crate) fn get_route_path_inner(
    route_id: u32,
    data: &RoutesData,
) -> Result<RoutePath, AppError> {
    let route = data
        .routes_by_id
        .get(&route_id)
        .ok_or_else(|| AppError::NotFound(format!("Route {} not found", route_id)))?;

    let mut shape_paths = Vec::with_capacity(route.shapes.len());

    for shape_id in &route.shapes {
        if let Some(points) = data.shapes_by_id.get(shape_id) {
            let path = points
                .iter()
                .map(|(lat, lon)| dequantize_coord(*lat, *lon))
                .collect();
            shape_paths.push(path);
        }
    }

    Ok(RoutePath {
        route_id: route.id,
        short_name: route.short_name.clone(),
        shape_paths,
    })
}

#[cfg(not(tarpaulin_include))]
#[tauri::command]
pub fn get_route_path(
    route_id: u32,
    routes_data: tauri::State<Arc<RoutesData>>,
) -> Result<RoutePath, AppError> {
    get_route_path_inner(route_id, &routes_data)
}

pub(crate) fn get_routes_for_stop_inner(
    stop_id: u32,
    data: &RoutesData,
) -> Result<Vec<RouteSummary>, AppError> {
    let route_ids = data
        .stop_to_routes
        .get(&stop_id)
        .ok_or_else(|| AppError::NotFound(format!("Stop {} not found", stop_id)))?;

    let routes = route_ids
        .iter()
        .filter_map(|route_id| data.route_summary_from_id(*route_id))
        .collect();

    Ok(routes)
}

#[cfg(not(tarpaulin_include))]
#[tauri::command]
pub fn get_routes_for_stop(
    stop_id: u32,
    routes_data: tauri::State<Arc<RoutesData>>,
) -> Result<Vec<RouteSummary>, AppError> {
    get_routes_for_stop_inner(stop_id, &routes_data)
}

pub(crate) fn get_nearby_stops_with_routes_inner(
    lat: f64,
    lon: f64,
    max_distance_meters: Option<f64>,
    limit: Option<usize>,
    data: &RoutesData,
) -> Result<Vec<NearbyStop>, AppError> {
    let mut stops: Vec<NearbyStop> = data
        .stop_coords
        .iter()
        .map(|(stop_id, (q_lat, q_lon))| {
            let coord = dequantize_coord(*q_lat, *q_lon);
            let distance_meters = haversine_meters(lat, lon, coord.lat, coord.lon);

            NearbyStop {
                stop_id: *stop_id,
                lat: coord.lat,
                lon: coord.lon,
                distance_meters,
                routes: collect_stop_routes(data, *stop_id),
            }
        })
        .collect();

    if let Some(max_distance) = max_distance_meters {
        stops.retain(|stop| stop.distance_meters <= max_distance);
    }

    stops.sort_by(|a, b| a.distance_meters.total_cmp(&b.distance_meters));

    if let Some(max_items) = limit {
        stops.truncate(max_items);
    }

    Ok(stops)
}

#[cfg(not(tarpaulin_include))]
#[tauri::command]
pub fn get_nearby_stops_with_routes(
    lat: f64,
    lon: f64,
    max_distance_meters: Option<f64>,
    limit: Option<usize>,
    routes_data: tauri::State<Arc<RoutesData>>,
) -> Result<Vec<NearbyStop>, AppError> {
    get_nearby_stops_with_routes_inner(lat, lon, max_distance_meters, limit, &routes_data)
}

pub(crate) fn get_stops_for_route_short_name_inner(
    short_name: &str,
    data: &RoutesData,
) -> Result<Vec<DatasetStop>, AppError> {
    let route_ids = match data.route_ids_by_short_name.get(short_name) {
        Some(ids) => ids,
        None => return Ok(Vec::new()),
    };

    let route_ids_set: std::collections::HashSet<u32> = route_ids.iter().copied().collect();

    let mut stops = Vec::new();
    for (stop_id, stop_route_ids) in &data.stop_to_routes {
        if stop_route_ids
            .iter()
            .any(|route_id| route_ids_set.contains(route_id))
        {
            if let Some((q_lat, q_lon)) = data.stop_coords.get(stop_id) {
                let coord = dequantize_coord(*q_lat, *q_lon);
                stops.push(DatasetStop {
                    stop_id: *stop_id,
                    lat: coord.lat,
                    lon: coord.lon,
                });
            }
        }
    }

    Ok(stops)
}

#[cfg(not(tarpaulin_include))]
#[tauri::command]
pub fn get_stops_for_route_short_name(
    short_name: String,
    routes_data: tauri::State<Arc<RoutesData>>,
) -> Result<Vec<DatasetStop>, AppError> {
    get_stops_for_route_short_name_inner(&short_name, &routes_data)
}

pub(crate) fn get_all_route_paths_by_mode_inner(
    mode_type: u16,
    data: &RoutesData,
) -> Result<Vec<RoutePath>, AppError> {
    let mode = RouteMode::from(mode_type);
    let route_ids = data
        .route_ids_by_mode
        .get(&mode)
        .cloned()
        .unwrap_or_default();

    let result = route_ids
        .into_iter()
        .filter_map(|route_id| {
            let route = data.routes_by_id.get(&route_id)?;
            let points = longest_shape_points(route, data)?;

            let path: Vec<Coordinate> = points
                .iter()
                .step_by(ROUTE_PATH_SAMPLE_STEP)
                .map(|(lat, lon)| dequantize_coord(*lat, *lon))
                .collect();

            if path.is_empty() {
                return None;
            }

            Some(RoutePath {
                route_id: route.id,
                short_name: route.short_name.clone(),
                shape_paths: vec![path],
            })
        })
        .collect();

    Ok(result)
}

#[cfg(not(tarpaulin_include))]
#[tauri::command]
pub fn get_all_route_paths_by_mode(
    mode_type: u16,
    routes_data: tauri::State<Arc<RoutesData>>,
) -> Result<Vec<RoutePath>, AppError> {
    get_all_route_paths_by_mode_inner(mode_type, &routes_data)
}

#[derive(Serialize)]
pub struct DatasetStats {
    pub stops: usize,
    pub routes: usize,
    pub shapes: usize,
    pub stop_routes: usize,
}

pub(crate) fn get_dataset_stats_inner(data: &RoutesData) -> DatasetStats {
    DatasetStats {
        stops: data.stop_coords.len(),
        routes: data.routes_by_id.len(),
        shapes: data.shapes_by_id.len(),
        stop_routes: data.stop_to_routes.len(),
    }
}

#[cfg(not(tarpaulin_include))]
#[tauri::command]
pub fn get_dataset_stats(routes_data: tauri::State<Arc<RoutesData>>) -> DatasetStats {
    get_dataset_stats_inner(&routes_data)
}

/// Shared CWD lock for all tests that manipulate the current working directory.
/// Must be shared so that parallel test threads don't race on CWD changes.
#[cfg(test)]
pub(crate) fn shared_cwd_lock() -> &'static std::sync::Mutex<()> {
    static LOCK: std::sync::OnceLock<std::sync::Mutex<()>> = std::sync::OnceLock::new();
    LOCK.get_or_init(|| std::sync::Mutex::new(()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dequantize_boundaries() {
        assert_eq!(dequantize(0, 59.5, 61.0), 59.5);
        assert!((dequantize(65535, 59.5, 61.0) - 61.0).abs() < 1e-9);
    }

    #[test]
    fn test_dequantize_midpoint() {
        let mid = dequantize(32767, 0.0, 100.0);
        assert!((mid - 50.0).abs() < 0.01);
    }

    #[test]
    fn test_dequantize_coord_produces_valid_range() {
        let coord = dequantize_coord(32767, 32767);
        assert!(coord.lat >= FIN_MIN_LAT && coord.lat <= FIN_MAX_LAT);
        assert!(coord.lon >= FIN_MIN_LON && coord.lon <= FIN_MAX_LON);
    }

    #[test]
    fn test_haversine_same_point_is_zero() {
        let d = haversine_meters(60.17, 24.94, 60.17, 24.94);
        assert!(d < 1e-6);
    }

    #[test]
    fn test_haversine_known_distance() {
        // Roughly 1 degree of latitude ≈ 111 km near Helsinki
        let d = haversine_meters(60.0, 25.0, 61.0, 25.0);
        assert!(d > 110_000.0 && d < 113_000.0);
    }

    #[test]
    fn test_route_mode_from_known_types() {
        assert_eq!(RouteMode::from(0), RouteMode::Tram);
        assert_eq!(RouteMode::from(1), RouteMode::Subway);
        assert_eq!(RouteMode::from(2), RouteMode::Rail);
        assert_eq!(RouteMode::from(3), RouteMode::Bus);
        assert_eq!(RouteMode::from(4), RouteMode::Ferry);
        assert_eq!(RouteMode::from(109), RouteMode::SuburbanRail);
        assert_eq!(RouteMode::from(701), RouteMode::RegionalBus);
        assert_eq!(RouteMode::from(702), RouteMode::ExpressBus);
        assert_eq!(RouteMode::from(704), RouteMode::LocalBus);
        assert_eq!(RouteMode::from(900), RouteMode::TramService);
    }

    #[test]
    fn test_route_mode_from_remaining_types() {
        assert_eq!(RouteMode::from(5), RouteMode::CableTram);
        assert_eq!(RouteMode::from(6), RouteMode::AerialLift);
        assert_eq!(RouteMode::from(7), RouteMode::Funicular);
        assert_eq!(RouteMode::from(11), RouteMode::Trolleybus);
        assert_eq!(RouteMode::from(12), RouteMode::Monorail);
        assert_eq!(RouteMode::from(700), RouteMode::Coach);
    }

    #[test]
    fn test_route_mode_from_unknown() {
        let mode = RouteMode::from(42);
        assert_eq!(mode, RouteMode::Unknown(42));
    }

    #[test]
    fn test_route_mode_unknown_preserves_value() {
        assert_eq!(RouteMode::from(999), RouteMode::Unknown(999));
        assert_eq!(RouteMode::from(0xFFFF), RouteMode::Unknown(0xFFFF));
    }

    fn make_test_routes_data() -> RoutesData {
        let mut routes_by_id = std::collections::HashMap::new();
        let route = RouteSummaryInternal {
            id: 1,
            short_name: "7A".to_string(),
            mode_type: RouteMode::Tram,
            shapes: vec![10],
        };
        routes_by_id.insert(1u32, route);

        let mut route_ids_by_short_name = std::collections::HashMap::new();
        route_ids_by_short_name.insert("7A".to_string(), vec![1u32]);

        let mut route_ids_by_mode = std::collections::HashMap::new();
        route_ids_by_mode.insert(RouteMode::Tram, vec![1u32]);

        let mut shapes_by_id = std::collections::HashMap::new();
        shapes_by_id.insert(10u32, vec![(32767u16, 32767u16), (33000u16, 33000u16)]);

        let mut stop_coords = std::collections::HashMap::new();
        stop_coords.insert(100u32, (32767u16, 32767u16));
        stop_coords.insert(101u32, (33000u16, 33000u16));

        let mut stop_to_routes = std::collections::HashMap::new();
        stop_to_routes.insert(100u32, vec![1u32]);
        stop_to_routes.insert(101u32, vec![1u32]);

        RoutesData {
            routes_by_id,
            route_ids_by_short_name,
            route_ids_by_mode,
            shapes_by_id,
            stop_coords,
            stop_to_routes,
        }
    }

    #[test]
    fn test_route_summary_from_id_found() {
        let data = make_test_routes_data();
        let summary = data.route_summary_from_id(1);
        assert!(summary.is_some());
        let s = summary.unwrap();
        assert_eq!(s.route_id, 1);
        assert_eq!(s.short_name, "7A");
    }

    #[test]
    fn test_route_summary_from_id_not_found() {
        let data = make_test_routes_data();
        assert!(data.route_summary_from_id(999).is_none());
    }

    #[test]
    fn test_collect_stop_routes_found() {
        let data = make_test_routes_data();
        let routes = collect_stop_routes(&data, 100);
        assert_eq!(routes.len(), 1);
        assert_eq!(routes[0].short_name, "7A");
    }

    #[test]
    fn test_collect_stop_routes_not_found() {
        let data = make_test_routes_data();
        let routes = collect_stop_routes(&data, 999);
        assert!(routes.is_empty());
    }

    #[test]
    fn test_longest_shape_points_found() {
        let data = make_test_routes_data();
        let route = data.routes_by_id.get(&1).unwrap();
        let points = longest_shape_points(route, &data);
        assert!(points.is_some());
        assert_eq!(points.unwrap().len(), 2);
    }

    #[test]
    fn test_longest_shape_points_no_shapes() {
        let data = make_test_routes_data();
        let route = RouteSummaryInternal {
            id: 99,
            short_name: "X".to_string(),
            mode_type: RouteMode::Bus,
            shapes: vec![],
        };
        assert!(longest_shape_points(&route, &data).is_none());
    }

    #[test]
    fn test_from_dataset_builds_indexes() {
        use converter::{Route, Shape, StopRoutes, TransitDataset};

        let dataset = TransitDataset {
            stops: vec![(1, 32767, 32767)],
            routes: vec![Route {
                id: 10,
                short_name: "7A".to_string(),
                route_type: 0,
                shapes: vec![20],
            }],
            shapes: vec![Shape {
                id: 20,
                points: vec![(32767, 32767), (33000, 33000)],
            }],
            stop_routes: vec![StopRoutes {
                stop_id: 1,
                routes: vec![10],
            }],
        };

        let data = RoutesData::from_dataset(dataset);
        assert!(data.routes_by_id.contains_key(&10));
        assert!(data.route_ids_by_short_name.contains_key("7A"));
        assert!(data.route_ids_by_mode.contains_key(&RouteMode::Tram));
        assert!(data.shapes_by_id.contains_key(&20));
        assert!(data.stop_coords.contains_key(&1));
        assert!(data.stop_to_routes.contains_key(&1));
    }

    #[test]
    fn test_from_bytes_invalid_data_returns_error() {
        let result = RoutesData::from_bytes(b"not_valid_zstd");
        assert!(result.is_err());
    }

    #[test]
    fn test_from_bytes_valid_roundtrip() {
        use converter::{Route, Shape, StopRoutes, TransitDataset};

        let dataset = TransitDataset {
            stops: vec![(1, 32000, 32000)],
            routes: vec![Route {
                id: 1,
                short_name: "1".to_string(),
                route_type: 3,
                shapes: vec![1],
            }],
            shapes: vec![Shape {
                id: 1,
                points: vec![(32000, 32000)],
            }],
            stop_routes: vec![StopRoutes {
                stop_id: 1,
                routes: vec![1],
            }],
        };

        let encoded = bitcode::serialize(&dataset).unwrap();
        let compressed = zstd::encode_all(encoded.as_slice(), 3).unwrap();
        let data = RoutesData::from_bytes(&compressed).unwrap();
        assert!(data.routes_by_id.contains_key(&1));
    }

    #[test]
    fn test_load_missing_file_returns_io_error() {
        let _guard = shared_cwd_lock().lock().unwrap();

        let original = std::env::current_dir().unwrap();
        let tmp = std::env::temp_dir().join("routes_no_data_dir_test");
        std::fs::create_dir_all(&tmp).unwrap();
        std::env::set_current_dir(&tmp).unwrap();

        let result = RoutesData::load();
        std::env::set_current_dir(original).unwrap();

        assert!(matches!(result, Err(crate::error::AppError::Io(_))));
    }

    #[test]
    fn test_get_route_path_inner_found() {
        let data = make_test_routes_data();
        let result = get_route_path_inner(1, &data);
        assert!(result.is_ok());
        let path = result.unwrap();
        assert_eq!(path.route_id, 1);
        assert_eq!(path.short_name, "7A");
        assert_eq!(path.shape_paths.len(), 1);
        assert_eq!(path.shape_paths[0].len(), 2);
    }

    #[test]
    fn test_get_route_path_inner_not_found() {
        let data = make_test_routes_data();
        let result = get_route_path_inner(999, &data);
        assert!(matches!(result, Err(crate::error::AppError::NotFound(_))));
    }

    #[test]
    fn test_get_routes_for_stop_inner_found() {
        let data = make_test_routes_data();
        let result = get_routes_for_stop_inner(100, &data);
        assert!(result.is_ok());
        let routes = result.unwrap();
        assert_eq!(routes.len(), 1);
        assert_eq!(routes[0].short_name, "7A");
    }

    #[test]
    fn test_get_routes_for_stop_inner_not_found() {
        let data = make_test_routes_data();
        let result = get_routes_for_stop_inner(999, &data);
        assert!(matches!(result, Err(crate::error::AppError::NotFound(_))));
    }

    #[test]
    fn test_get_nearby_stops_with_routes_inner_no_limit() {
        let data = make_test_routes_data();
        let result = get_nearby_stops_with_routes_inner(60.17, 24.94, None, None, &data);
        assert!(result.is_ok());
        let stops = result.unwrap();
        assert_eq!(stops.len(), 2);
    }

    #[test]
    fn test_get_nearby_stops_with_routes_inner_with_distance_filter() {
        let data = make_test_routes_data();
        // Use a very small distance to filter all stops out
        let result = get_nearby_stops_with_routes_inner(60.17, 24.94, Some(0.001), None, &data);
        assert!(result.is_ok());
        let stops = result.unwrap();
        assert!(stops.is_empty());
    }

    #[test]
    fn test_get_nearby_stops_with_routes_inner_with_limit() {
        let data = make_test_routes_data();
        let result = get_nearby_stops_with_routes_inner(60.17, 24.94, None, Some(1), &data);
        assert!(result.is_ok());
        let stops = result.unwrap();
        assert_eq!(stops.len(), 1);
    }

    #[test]
    fn test_get_stops_for_route_short_name_inner_found() {
        let data = make_test_routes_data();
        let result = get_stops_for_route_short_name_inner("7A", &data);
        assert!(result.is_ok());
        let stops = result.unwrap();
        assert_eq!(stops.len(), 2);
    }

    #[test]
    fn test_get_stops_for_route_short_name_inner_unknown() {
        let data = make_test_routes_data();
        let result = get_stops_for_route_short_name_inner("UNKNOWN", &data);
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[test]
    fn test_get_all_route_paths_by_mode_inner_tram() {
        let data = make_test_routes_data();
        let result = get_all_route_paths_by_mode_inner(0, &data); // 0 = Tram
        assert!(result.is_ok());
        let paths = result.unwrap();
        assert_eq!(paths.len(), 1);
        assert_eq!(paths[0].short_name, "7A");
    }

    #[test]
    fn test_get_all_route_paths_by_mode_inner_no_such_mode() {
        let data = make_test_routes_data();
        let result = get_all_route_paths_by_mode_inner(4, &data); // 4 = Ferry, not in test data
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[test]
    fn test_get_dataset_stats_inner() {
        let data = make_test_routes_data();
        let stats = get_dataset_stats_inner(&data);
        assert_eq!(stats.routes, 1);
        assert_eq!(stats.shapes, 1);
        assert_eq!(stats.stops, 2);
        assert_eq!(stats.stop_routes, 2);
    }

    #[test]
    fn test_get_route_path_inner_shape_id_missing_in_map() {
        // Route's shape_id has no entry in shapes_by_id → that shape_path is silently skipped
        let mut data = make_test_routes_data();
        data.routes_by_id.insert(
            2,
            RouteSummaryInternal {
                id: 2,
                short_name: "X1".to_string(),
                mode_type: RouteMode::Bus,
                shapes: vec![99], // 99 is not in shapes_by_id
            },
        );
        let result = get_route_path_inner(2, &data);
        assert!(result.is_ok());
        let path = result.unwrap();
        assert_eq!(path.route_id, 2);
        // Shape 99 not found → shape silently omitted → empty shape_paths
        assert!(path.shape_paths.is_empty());
    }

    #[test]
    fn test_get_all_route_paths_by_mode_inner_empty_shape_excluded() {
        // A route with a shape that has 0 points produces an empty path → route is excluded
        let mut data = make_test_routes_data();
        data.routes_by_id.insert(
            2,
            RouteSummaryInternal {
                id: 2,
                short_name: "X2".to_string(),
                mode_type: RouteMode::Bus,
                shapes: vec![20],
            },
        );
        data.route_ids_by_mode
            .entry(RouteMode::Bus)
            .or_default()
            .push(2);
        data.shapes_by_id.insert(20, vec![]); // empty shape → path.is_empty() == true
        let result = get_all_route_paths_by_mode_inner(3, &data); // 3 = Bus
        assert!(result.is_ok());
        let paths = result.unwrap();
        // Route 2 has an empty shape → excluded from results
        assert!(paths.iter().all(|p| p.route_id != 2));
    }

    #[test]
    fn test_get_nearby_stops_with_routes_inner_sorted_by_distance() {
        let data = make_test_routes_data();
        let result = get_nearby_stops_with_routes_inner(60.17, 24.94, None, None, &data);
        assert!(result.is_ok());
        let stops = result.unwrap();
        // Result must be sorted ascending by distance_meters
        for window in stops.windows(2) {
            assert!(
                window[0].distance_meters <= window[1].distance_meters,
                "stops not sorted: {} > {}",
                window[0].distance_meters,
                window[1].distance_meters
            );
        }
    }

    #[test]
    fn test_get_stops_for_route_short_name_inner_stop_missing_coords() {
        // A stop that is in stop_to_routes for the route but has no entry in stop_coords
        // should be silently omitted from the result.
        let mut data = make_test_routes_data();
        // Stop 200 is linked to route 1 ("7A") but has no coordinates
        data.stop_to_routes.insert(200, vec![1]);
        // deliberately NOT adding stop 200 to stop_coords
        let result = get_stops_for_route_short_name_inner("7A", &data);
        assert!(result.is_ok());
        let stops = result.unwrap();
        // Stop 200 must not appear in results
        assert!(stops.iter().all(|s| s.stop_id != 200));
        // The two stops that do have coords should still be present
        assert_eq!(stops.len(), 2);
    }
}

/// Tests that exercise converter crate functions directly, ensuring coverage counts
/// for converter code when running `cargo tarpaulin -p hsl-routes`.
#[cfg(test)]
mod converter_coverage_tests {
    use converter::{
        build_and_serialize_addresses, process_geojson_line, quantize, Address, AddressPoint,
    };
    use rstar::Point;
    use std::fs;
    use std::path::Path;

    const HEL_MIN_LON: f64 = 24.7968139972634;
    const HEL_MAX_LON: f64 = 25.129150423044536;
    const HEL_MIN_LAT: f64 = 60.132332639707755;
    const HEL_MAX_LAT: f64 = 60.25384126877702;

    fn mid_lon() -> f64 {
        (HEL_MIN_LON + HEL_MAX_LON) / 2.0
    }
    fn mid_lat() -> f64 {
        (HEL_MIN_LAT + HEL_MAX_LAT) / 2.0
    }

    fn make_geojson(lon: f64, lat: f64) -> String {
        format!(
            r#"{{"type":"Feature","geometry":{{"type":"Point","coordinates":[{lon},{lat}]}},"properties":{{"street":"Testikatu","number":"1","city":"Helsinki","postcode":"00100"}}}}"#
        )
    }

    #[test]
    fn test_quantize_min_max_midpoint() {
        assert_eq!(quantize(0.0, 0.0, 1.0), 0);
        assert_eq!(quantize(1.0, 0.0, 1.0), 65535);
        let mid = quantize(0.5, 0.0, 1.0);
        assert!(mid > 32000 && mid < 33000);
    }

    #[test]
    fn test_quantize_helsinki_coords() {
        assert_eq!(quantize(HEL_MIN_LAT, HEL_MIN_LAT, HEL_MAX_LAT), 0);
        assert_eq!(quantize(HEL_MAX_LAT, HEL_MIN_LAT, HEL_MAX_LAT), 65535);
    }

    #[test]
    fn test_process_geojson_line_valid() {
        let line = make_geojson(mid_lon(), mid_lat());
        let result = process_geojson_line(&line, 5);
        assert!(result.is_some());
        let (addr, point) = result.unwrap();
        assert_eq!(addr.street, "Testikatu");
        assert_eq!(addr.city, "Helsinki");
        assert_eq!(point.index, 5);
    }

    #[test]
    fn test_process_geojson_line_outside_bounds() {
        // Tampere coordinates are outside Helsinki bounding box
        let line = make_geojson(23.761, 61.498);
        assert!(process_geojson_line(&line, 0).is_none());
    }

    #[test]
    fn test_process_geojson_line_malformed() {
        assert!(process_geojson_line("not json", 0).is_none());
    }

    #[test]
    fn test_address_point_trait_generate() {
        let point = <AddressPoint as Point>::generate(|i| if i == 0 { 24.9 } else { 60.2 });
        assert_eq!(point.nth(0), 24.9);
        assert_eq!(point.nth(1), 60.2);
        assert_eq!(point.index, 0);
    }

    #[test]
    fn test_address_point_trait_nth_mut() {
        let mut point = AddressPoint {
            point: [24.9f32, 60.2f32],
            index: 0,
        };
        *point.nth_mut(0) = 25.0;
        *point.nth_mut(1) = 60.25;
        assert_eq!(point.point[0], 25.0);
        assert_eq!(point.point[1], 60.25);
    }

    #[test]
    fn test_build_and_serialize_addresses() {
        let addr = Address {
            street: "Testikatu".to_string(),
            number: "1".to_string(),
            city: "Helsinki".to_string(),
            postcode: "00100".to_string(),
            lat: quantize(mid_lat(), HEL_MIN_LAT, HEL_MAX_LAT),
            lon: quantize(mid_lon(), HEL_MIN_LON, HEL_MAX_LON),
        };
        let point = AddressPoint {
            point: [mid_lon() as f32, mid_lat() as f32],
            index: 0,
        };
        let result = build_and_serialize_addresses(vec![addr], vec![point]);
        assert!(result.is_ok());
        assert!(!result.unwrap().is_empty());
    }

    fn write_file(path: &Path, content: &str) {
        fs::write(path, content).expect("failed to write test file");
    }

    #[test]
    fn test_build_transit_dataset_from_dir() {
        use converter::build_transit_dataset_from_dir;
        use std::time::{SystemTime, UNIX_EPOCH};

        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("hsl_routes_conv_test_{ts}"));
        fs::create_dir_all(&dir).unwrap();

        write_file(
            &dir.join("stops.txt"),
            "stop_id,stop_lat,stop_lon\nS1,60.17,24.94\nS2,60.20,24.97\n",
        );
        write_file(
            &dir.join("routes.txt"),
            "route_id,route_short_name,route_type\nR1,7A,0\n",
        );
        write_file(
            &dir.join("trips.txt"),
            // R1,T1,SH1: known route, new shape → None branch in shape_id_map (line 297)
            // R1,T2,SH1: known route, existing shape → Some branch in shape_id_map (line 296)
            // UNKNOWN,T99,SHX: unknown route → continue (line 292)
            "route_id,trip_id,shape_id\nR1,T1,SH1\nR1,T2,SH1\nUNKNOWN,T99,SHX\n",
        );
        write_file(&dir.join("stop_times.txt"), "trip_id,stop_id\nT1,S1\n");
        write_file(
            &dir.join("shapes.txt"),
            "shape_id,shape_pt_lat,shape_pt_lon,shape_pt_sequence\nSH1,60.19,24.91,1\nSH1,60.21,24.95,2\n",
        );

        let dataset = build_transit_dataset_from_dir(&dir).expect("dataset build failed");
        assert_eq!(dataset.stops.len(), 2);
        assert_eq!(dataset.routes.len(), 1);
        assert_eq!(dataset.routes[0].short_name, "7A");
        assert_eq!(dataset.routes[0].route_type, 0);
        assert_eq!(dataset.shapes.len(), 1);
        assert_eq!(dataset.shapes[0].points.len(), 2);

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_write_and_resolve_transit_dataset() {
        use converter::{
            resolve_default_hsl_input_dir, write_transit_dataset, Route, Shape, StopRoutes,
            TransitDataset,
        };
        use std::time::{SystemTime, UNIX_EPOCH};

        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("hsl_routes_write_test_{ts}"));
        fs::create_dir_all(&dir).unwrap();
        let out_path = dir.join("output.bin.zst");

        let dataset = TransitDataset {
            stops: vec![(0, 100, 200)],
            routes: vec![Route {
                id: 0,
                short_name: "1".to_string(),
                route_type: 3,
                shapes: vec![0],
            }],
            shapes: vec![Shape {
                id: 0,
                points: vec![(100, 200)],
            }],
            stop_routes: vec![StopRoutes {
                stop_id: 0,
                routes: vec![0],
            }],
        };

        write_transit_dataset(&dataset, &out_path).expect("write failed");
        assert!(out_path.exists());

        // resolve_default_hsl_input_dir returns a PathBuf (coverage for that fn)
        let _resolved = resolve_default_hsl_input_dir();

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_resolve_default_hsl_input_dir_with_existing_direct() {
        use converter::resolve_default_hsl_input_dir;
        use std::time::{SystemTime, UNIX_EPOCH};

        let _guard = super::shared_cwd_lock().lock().unwrap();
        let original = std::env::current_dir().unwrap();

        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let tmp = std::env::temp_dir().join(format!("conv_resolve_direct_{ts}"));
        let direct = tmp.join("data").join("hsl-GTFS-2026-03-12");
        fs::create_dir_all(&direct).unwrap();

        std::env::set_current_dir(&tmp).unwrap();
        let result = resolve_default_hsl_input_dir();
        std::env::set_current_dir(original).unwrap();

        assert_eq!(result, std::path::PathBuf::from("data/hsl-GTFS-2026-03-12"));

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_load_addresses_valid_and_invalid_lines() {
        use converter::load_addresses;
        use std::time::{SystemTime, UNIX_EPOCH};

        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("hsl_routes_load_addr_{ts}"));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("addresses.ndgeojson");

        let valid = make_geojson(mid_lon(), mid_lat());
        let outside = make_geojson(23.761, 61.498); // Outside Helsinki
        fs::write(&path, format!("{valid}\nnot-json\n{outside}\n")).unwrap();

        let (addresses, points) = load_addresses(path.to_str().unwrap()).unwrap();
        assert_eq!(addresses.len(), 1);
        assert_eq!(points.len(), 1);

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_build_transit_dataset_with_orphan_shape_in_shapes_file() {
        use converter::build_transit_dataset_from_dir;
        use std::time::{SystemTime, UNIX_EPOCH};

        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("hsl_routes_orphan_shape_{ts}"));
        fs::create_dir_all(&dir).unwrap();

        write_file(
            &dir.join("stops.txt"),
            "stop_id,stop_lat,stop_lon\nS1,60.17,24.94\n",
        );
        write_file(
            &dir.join("routes.txt"),
            "route_id,route_short_name,route_type\nR1,7A,0\n",
        );
        write_file(
            &dir.join("trips.txt"),
            "route_id,trip_id,shape_id\nR1,T1,SH1\n",
        );
        write_file(&dir.join("stop_times.txt"), "trip_id,stop_id\nT1,S1\n");
        // SH2 appears only in shapes.txt (not in trips.txt) → orphan shape None branch
        write_file(
            &dir.join("shapes.txt"),
            "shape_id,shape_pt_lat,shape_pt_lon,shape_pt_sequence\nSH1,60.19,24.91,1\nSH2,60.20,24.92,1\n",
        );

        let dataset = build_transit_dataset_from_dir(&dir).expect("dataset build failed");
        // Both SH1 (from trips) and SH2 (orphan) should appear in shapes
        assert_eq!(dataset.shapes.len(), 2);

        let _ = fs::remove_dir_all(&dir);
    }
}

/// Tests that load file routes data from disk
#[cfg(test)]
mod load_routes_filesystem_tests {
    use super::*;

    #[test]
    fn test_load_routes_from_written_file() {
        let _guard = shared_cwd_lock().lock().unwrap();

        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("routes_load_test_{ts}"));
        std::fs::create_dir_all(&dir).unwrap();
        let data_dir = dir.join("data");
        std::fs::create_dir_all(&data_dir).unwrap();

        use converter::{Route, Shape, StopRoutes, TransitDataset};
        let dataset = TransitDataset {
            stops: vec![(1, 32000, 32000)],
            routes: vec![Route {
                id: 1,
                short_name: "99X".to_string(),
                route_type: 3,
                shapes: vec![1],
            }],
            shapes: vec![Shape {
                id: 1,
                points: vec![(32000, 32000)],
            }],
            stop_routes: vec![StopRoutes {
                stop_id: 1,
                routes: vec![1],
            }],
        };
        let encoded = bitcode::serialize(&dataset).unwrap();
        let compressed = zstd::encode_all(encoded.as_slice(), 3).unwrap();
        let out = data_dir.join("routes.bin.zst");
        std::fs::write(&out, &compressed).unwrap();

        let original = std::env::current_dir().unwrap();
        std::env::set_current_dir(&dir).unwrap();
        let result = RoutesData::load();
        std::env::set_current_dir(original).unwrap();

        assert!(result.is_ok());
        let data = result.unwrap();
        assert!(data.routes_by_id.contains_key(&1));

        let _ = std::fs::remove_dir_all(&dir);
    }
}
