use anyhow::Result;
use rstar::{Point, RTree};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

// Bounding box constants for Helsinki area (address dataset)
const HEL_MIN_LON: f64 = 24.7968139972634;
const HEL_MAX_LON: f64 = 25.129150423044536;
const HEL_MIN_LAT: f64 = 60.132332639707755;
const HEL_MAX_LAT: f64 = 60.25384126877702;

// Bounding box constants for Finland area (GTFS transit dataset)
const FIN_MIN_LAT: f64 = 59.5;
const FIN_MAX_LAT: f64 = 61.0;
const FIN_MIN_LON: f64 = 23.0;
const FIN_MAX_LON: f64 = 26.0;

/// A Helsinki address record with coordinates quantized to u16 for compact binary storage.
/// Lat/lon are normalized against the Helsinki bounding box via `quantize()`.
#[derive(Serialize, Deserialize, Clone)]
pub struct Address {
    pub street: String,
    pub number: String,
    pub city: String,
    pub postcode: String,
    /// Quantized latitude within HEL_MIN_LAT..HEL_MAX_LAT
    pub lat: u16,
    /// Quantized longitude within HEL_MIN_LON..HEL_MAX_LON
    pub lon: u16,
}

/// A spatial point stored in the R-tree index, holding full-precision f32 [lon, lat].
/// `index` references the corresponding entry in the flat `Address` vec.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct AddressPoint {
    /// [longitude, latitude] in WGS-84 as f32
    pub point: [f32; 2],
    /// Index into the parallel `addresses` Vec
    pub index: u32,
}

impl Point for AddressPoint {
    type Scalar = f32;
    const DIMENSIONS: usize = 2;

    fn generate(mut generator: impl FnMut(usize) -> Self::Scalar) -> Self {
        AddressPoint {
            point: [generator(0), generator(1)],
            index: 0,
        }
    }

    fn nth(&self, index: usize) -> Self::Scalar {
        self.point[index]
    }

    fn nth_mut(&mut self, index: usize) -> &mut Self::Scalar {
        &mut self.point[index]
    }
}

/// A GTFS shape (polyline) with points quantized to u16 against the Finland bounding box.
#[derive(Serialize, Deserialize)]
pub struct Shape {
    pub id: u32,
    /// Ordered sequence of (quantized_lat, quantized_lon) pairs
    pub points: Vec<(u16, u16)>,
}

/// A GTFS route with its display name, type, and associated shape IDs.
/// `route_type` follows the GTFS spec (0 = tram, 1 = metro, 3 = bus, etc.).
#[derive(Serialize, Deserialize)]
pub struct Route {
    pub id: u32,
    pub short_name: String,
    /// GTFS route_type stored as u16 (preserves extended values like 700+)
    pub route_type: u16,
    /// Shape IDs drawn for this route
    pub shapes: Vec<u32>,
}

/// Maps a single stop to all route IDs that serve it.
#[derive(Serialize, Deserialize)]
pub struct StopRoutes {
    pub stop_id: u32,
    /// Sorted list of route IDs passing through this stop
    pub routes: Vec<u32>,
}

/// The complete processed GTFS dataset serialised and zstd-compressed for runtime use.
#[derive(Serialize, Deserialize)]
pub struct TransitDataset {
    /// Each stop as (id, quantized_lat, quantized_lon) against the Finland bounding box
    pub stops: Vec<(u32, u16, u16)>,
    pub routes: Vec<Route>,
    /// Sorted by Shape::id
    pub shapes: Vec<Shape>,
    /// Sorted by StopRoutes::stop_id
    pub stop_routes: Vec<StopRoutes>,
}

/// Raw CSV row from stops.txt; f64 coordinates are quantized to u16 after loading.
#[derive(Deserialize)]
struct StopRow {
    stop_id: String,
    stop_lat: f64,
    stop_lon: f64,
}

/// Raw CSV row from routes.txt.
#[derive(Deserialize)]
struct RouteRow {
    route_id: String,
    route_short_name: String,
    route_type: u32,
}

/// Raw CSV row from trips.txt; used to link routes → shapes and trips → stops.
#[derive(Deserialize)]
struct TripRow {
    route_id: String,
    trip_id: String,
    shape_id: String,
}

/// Minimal subset of stop_times.txt needed to map stops to routes via trip_id.
#[derive(Deserialize)]
struct StopTimeRow {
    trip_id: String,
    stop_id: String,
}

/// Raw CSV row from shapes.txt; f64 coordinates are quantized to u16 after loading.
#[derive(Deserialize)]
struct ShapeRow {
    shape_id: String,
    shape_pt_lat: f64,
    shape_pt_lon: f64,
    shape_pt_sequence: u32,
}

/// Normalizes `value` within `[min, max]` and maps it linearly to the full u16 range `[0, 65535]`.
/// Values outside the range are not clamped — callers should pre-filter if needed.
pub fn quantize(value: f64, min: f64, max: f64) -> u16 {
    let normalized = (value - min) / (max - min);
    (normalized * 65535.0).round() as u16
}

/// Parses a single GeoJSON feature line from the address dataset.
///
/// Returns `None` if the line is malformed or the coordinates fall outside the Helsinki bounding box.
/// `next_index` becomes the `AddressPoint::index` linking the spatial point back to its `Address`.
pub fn process_geojson_line(line: &str, next_index: u32) -> Option<(Address, AddressPoint)> {
    let geo: serde_json::Value = match serde_json::from_str(line) {
        Ok(value) => value,
        Err(_) => return None,
    };

    let coords = &geo["geometry"]["coordinates"];
    let lon = coords[0].as_f64()?;
    let lat = coords[1].as_f64()?;

    if !(HEL_MIN_LON..=HEL_MAX_LON).contains(&lon) || !(HEL_MIN_LAT..=HEL_MAX_LAT).contains(&lat) {
        return None;
    }

    let qlon = quantize(lon, HEL_MIN_LON, HEL_MAX_LON);
    let qlat = quantize(lat, HEL_MIN_LAT, HEL_MAX_LAT);

    let props = &geo["properties"];

    let addr = Address {
        street: props["street"]
            .as_str()
            .map_or_else(String::new, ToString::to_string),
        number: props["number"]
            .as_str()
            .map_or_else(String::new, ToString::to_string),
        city: props["city"]
            .as_str()
            .map_or_else(String::new, ToString::to_string),
        postcode: props["postcode"]
            .as_str()
            .map_or_else(String::new, ToString::to_string),
        lat: qlat,
        lon: qlon,
    };

    let point = AddressPoint {
        point: [lon as f32, lat as f32],
        index: next_index,
    };

    Some((addr, point))
}

/// Reads a newline-delimited GeoJSON file at `path` and returns all valid Helsinki addresses
/// together with their matching spatial index points.
///
/// Lines that are malformed or outside the Helsinki bounding box are silently skipped.
pub fn load_addresses(path: &str) -> Result<(Vec<Address>, Vec<AddressPoint>), std::io::Error> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);

    let mut addresses = Vec::new();
    let mut points = Vec::new();

    for line in reader.lines() {
        let line = line?;
        let index = addresses.len() as u32;

        if let Some((addr, point)) = process_geojson_line(&line, index) {
            addresses.push(addr);
            points.push(point);
        }
    }

    Ok((addresses, points))
}

/// Loads `points` into an R-tree, bundles it with `addresses`, serializes with `bitcode`,
/// and compresses the result with zstd at level 5.
///
/// The returned bytes are ready to be written directly to a `.bin` asset file.
pub fn build_and_serialize_addresses(
    addresses: Vec<Address>,
    points: Vec<AddressPoint>,
) -> Result<Vec<u8>> {
    let rtree = RTree::bulk_load(points);
    let output = (rtree, addresses);
    let encoded = bitcode::serialize(&output)?;
    let compressed = zstd::encode_all(&encoded[..], 14)?;
    Ok(compressed)
}

/// Builds a [`TransitDataset`] by reading the standard GTFS text files from `input_dir`
/// (`stops.txt`, `routes.txt`, `trips.txt`, `stop_times.txt`, `shapes.txt`).
///
/// String IDs from GTFS are replaced with compact `u32` sequential IDs, and all
/// coordinates are quantized to `u16` against the Finland bounding box.
pub fn build_transit_dataset_from_dir(input_dir: &Path) -> Result<TransitDataset> {
    let mut stop_id_map = HashMap::new();
    let mut route_id_map = HashMap::new();
    let mut shape_id_map = HashMap::new();

    let mut stops = Vec::new();

    let stops_path = input_dir.join("stops.txt");
    let mut rdr = csv::Reader::from_path(stops_path)?;
    for row in rdr.deserialize::<StopRow>() {
        let row = row?;
        let id = stop_id_map.len() as u32;
        stop_id_map.insert(row.stop_id, id);

        stops.push((
            id,
            quantize(row.stop_lat, FIN_MIN_LAT, FIN_MAX_LAT),
            quantize(row.stop_lon, FIN_MIN_LON, FIN_MAX_LON),
        ));
    }

    let mut routes = Vec::new();

    let routes_path = input_dir.join("routes.txt");
    let mut rdr = csv::Reader::from_path(routes_path)?;
    for row in rdr.deserialize::<RouteRow>() {
        let row = row?;

        let id = route_id_map.len() as u32;
        route_id_map.insert(row.route_id, id);

        routes.push(Route {
            id,
            short_name: row.route_short_name,
            route_type: row.route_type.min(u16::MAX as u32) as u16,
            shapes: Vec::new(),
        });
    }

    let mut trip_to_route = HashMap::new();
    let mut route_to_shapes: HashMap<u32, HashSet<u32>> = HashMap::new();

    let trips_path = input_dir.join("trips.txt");
    let mut rdr = csv::Reader::from_path(trips_path)?;
    for row in rdr.deserialize::<TripRow>() {
        let row = row?;

        let Some(&route) = route_id_map.get(&row.route_id) else {
            continue;
        };

        let shape = match shape_id_map.get(&row.shape_id) {
            Some(id) => *id,
            None => {
                let id = shape_id_map.len() as u32;
                shape_id_map.insert(row.shape_id.clone(), id);
                id
            }
        };

        trip_to_route.insert(row.trip_id, route);
        route_to_shapes.entry(route).or_default().insert(shape);
    }

    let mut stop_routes: HashMap<u32, HashSet<u32>> = HashMap::new();

    let stop_times_path = input_dir.join("stop_times.txt");
    let mut rdr = csv::Reader::from_path(stop_times_path)?;
    for row in rdr.deserialize::<StopTimeRow>() {
        let row = row?;

        if let Some(&stop) = stop_id_map.get(&row.stop_id) {
            if let Some(&route) = trip_to_route.get(&row.trip_id) {
                stop_routes.entry(stop).or_default().insert(route);
            }
        }
    }

    let mut shapes_tmp: HashMap<u32, Vec<(u32, u16, u16)>> = HashMap::new();

    let shapes_path = input_dir.join("shapes.txt");
    let mut rdr = csv::Reader::from_path(shapes_path)?;
    for row in rdr.deserialize::<ShapeRow>() {
        let row = row?;

        let shape_id = match shape_id_map.get(&row.shape_id) {
            Some(id) => *id,
            None => {
                let id = shape_id_map.len() as u32;
                shape_id_map.insert(row.shape_id.clone(), id);
                id
            }
        };

        shapes_tmp.entry(shape_id).or_default().push((
            row.shape_pt_sequence,
            quantize(row.shape_pt_lat, FIN_MIN_LAT, FIN_MAX_LAT),
            quantize(row.shape_pt_lon, FIN_MIN_LON, FIN_MAX_LON),
        ));
    }

    let mut shapes = Vec::new();
    for (id, mut pts) in shapes_tmp {
        pts.sort_by_key(|p| p.0);
        let points = pts.into_iter().map(|(_, lat, lon)| (lat, lon)).collect();
        shapes.push(Shape { id, points });
    }
    shapes.sort_by_key(|shape| shape.id);

    for route in &mut routes {
        if let Some(shape_ids) = route_to_shapes.get(&route.id) {
            route.shapes = shape_ids.iter().copied().collect();
            route.shapes.sort_unstable();
        }
    }

    let mut stop_routes_vec: Vec<StopRoutes> = stop_routes
        .into_iter()
        .map(|(stop_id, routes)| {
            let mut route_ids: Vec<u32> = routes.into_iter().collect();
            route_ids.sort_unstable();
            StopRoutes {
                stop_id,
                routes: route_ids,
            }
        })
        .collect();
    stop_routes_vec.sort_by_key(|entry| entry.stop_id);

    Ok(TransitDataset {
        stops,
        routes,
        shapes,
        stop_routes: stop_routes_vec,
    })
}

/// Serializes `dataset` with `bitcode` and writes it zstd-compressed (level 19) to `output_path`.
pub fn write_transit_dataset(dataset: &TransitDataset, output_path: &Path) -> Result<()> {
    let encoded = bitcode::serialize(dataset)?;
    let compressed = zstd::encode_all(&encoded[..], 14)?;
    std::fs::write(output_path, compressed)?;
    Ok(())
}

/// Returns the default GTFS input directory, trying `data/hsl-GTFS-2026-03-12` first
/// (when run from the converter crate root) and falling back to `../data/hsl-GTFS-2026-03-12`
/// (when run from the workspace root).
pub fn resolve_default_hsl_input_dir() -> PathBuf {
    let direct = PathBuf::from("data/hsl-GTFS-2026-03-12");
    if direct.exists() {
        return direct;
    }

    PathBuf::from("../data/hsl-GTFS-2026-03-12")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::Path;
    use std::sync::{Mutex, OnceLock};
    use std::time::{SystemTime, UNIX_EPOCH};

    fn cwd_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    fn unique_temp_dir(prefix: &str) -> PathBuf {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time went backwards")
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("converter_{prefix}_{now}"));
        fs::create_dir_all(&dir).expect("failed to create temporary directory");
        dir
    }

    fn write_file(path: &Path, content: &str) {
        fs::write(path, content).expect("failed to write test file");
    }

    // --- quantize ---

    #[test]
    fn quantize_min_value_yields_zero() {
        assert_eq!(quantize(0.0, 0.0, 1.0), 0);
    }

    #[test]
    fn quantize_max_value_yields_u16_max() {
        assert_eq!(quantize(1.0, 0.0, 1.0), 65535);
    }

    #[test]
    fn quantize_midpoint_yields_half_range() {
        let mid = quantize(0.5, 0.0, 1.0);
        // 0.5 * 65535 rounds to 32768
        assert_eq!(mid, 32768);
    }

    #[test]
    fn quantize_helsinki_lat_min() {
        assert_eq!(quantize(HEL_MIN_LAT, HEL_MIN_LAT, HEL_MAX_LAT), 0);
    }

    #[test]
    fn quantize_helsinki_lat_max() {
        assert_eq!(quantize(HEL_MAX_LAT, HEL_MIN_LAT, HEL_MAX_LAT), 65535);
    }

    // --- process_geojson_line ---

    fn make_geojson(lon: f64, lat: f64) -> String {
        format!(
            r#"{{"type":"Feature","geometry":{{"type":"Point","coordinates":[{lon},{lat}]}},"properties":{{"street":"Testikatu","number":"1","city":"Helsinki","postcode":"00100"}}}}"#
        )
    }

    #[test]
    fn process_geojson_line_valid_helsinki_coords() {
        // Centre of the Helsinki bounding box
        let lon = (HEL_MIN_LON + HEL_MAX_LON) / 2.0;
        let lat = (HEL_MIN_LAT + HEL_MAX_LAT) / 2.0;
        let line = make_geojson(lon, lat);

        let result = process_geojson_line(&line, 0);
        assert!(result.is_some());

        let (addr, point) = match result {
            Some(value) => value,
            None => panic!("expected parsed GeoJSON line"),
        };
        assert_eq!(addr.street, "Testikatu");
        assert_eq!(addr.city, "Helsinki");
        assert_eq!(point.index, 0);
        assert!((point.point[0] - lon as f32).abs() < 1e-5);
        assert!((point.point[1] - lat as f32).abs() < 1e-5);
    }

    #[test]
    fn process_geojson_line_outside_bounds_returns_none() {
        // Clearly outside Helsinki (somewhere in Tampere)
        let line = make_geojson(23.7610, 61.4978);
        assert!(process_geojson_line(&line, 0).is_none());
    }

    #[test]
    fn process_geojson_line_malformed_json_returns_none() {
        assert!(process_geojson_line("not json at all", 0).is_none());
    }

    #[test]
    fn process_geojson_line_index_is_preserved() {
        let lon = (HEL_MIN_LON + HEL_MAX_LON) / 2.0;
        let lat = (HEL_MIN_LAT + HEL_MAX_LAT) / 2.0;
        let line = make_geojson(lon, lat);

        let result = process_geojson_line(&line, 42);
        let (_, point) = match result {
            Some(value) => value,
            None => panic!("expected parsed GeoJSON line"),
        };
        assert_eq!(point.index, 42);
    }

    #[test]
    fn process_geojson_line_quantized_coords_in_range() {
        let lon = (HEL_MIN_LON + HEL_MAX_LON) / 2.0;
        let lat = (HEL_MIN_LAT + HEL_MAX_LAT) / 2.0;
        let line = make_geojson(lon, lat);

        let result = process_geojson_line(&line, 0);
        let (addr, _) = match result {
            Some(value) => value,
            None => panic!("expected parsed GeoJSON line"),
        };
        // Mid-point should quantize to roughly half of u16 range
        assert!(addr.lat > 30_000 && addr.lat < 35_000);
        assert!(addr.lon > 30_000 && addr.lon < 35_000);
    }

    // --- build_and_serialize_addresses ---

    #[test]
    fn build_and_serialize_addresses_produces_nonempty_bytes() {
        let lon = (HEL_MIN_LON + HEL_MAX_LON) / 2.0;
        let lat = (HEL_MIN_LAT + HEL_MAX_LAT) / 2.0;
        let addr = Address {
            street: "Testikatu".into(),
            number: "1".into(),
            city: "Helsinki".into(),
            postcode: "00100".into(),
            lat: quantize(lat, HEL_MIN_LAT, HEL_MAX_LAT),
            lon: quantize(lon, HEL_MIN_LON, HEL_MAX_LON),
        };
        let point = AddressPoint {
            point: [lon as f32, lat as f32],
            index: 0,
        };

        let result = build_and_serialize_addresses(vec![addr], vec![point]);
        assert!(result.is_ok());
        let bytes = match result {
            Ok(bytes) => bytes,
            Err(err) => panic!("expected serialized bytes, got error: {err}"),
        };
        assert!(!bytes.is_empty());
    }

    #[test]
    fn address_point_point_trait_methods_work() {
        let mut point = <AddressPoint as Point>::generate(|idx| if idx == 0 { 24.9 } else { 60.2 });
        assert_eq!(point.nth(0), 24.9);
        assert_eq!(point.nth(1), 60.2);

        *point.nth_mut(0) = 25.0;
        *point.nth_mut(1) = 60.25;
        assert_eq!(point.point, [25.0, 60.25]);
    }

    #[test]
    fn load_addresses_reads_valid_lines_and_skips_invalid() {
        let dir = unique_temp_dir("load_addresses_ok");
        let file = dir.join("addresses.ndgeojson");

        let lon = (HEL_MIN_LON + HEL_MAX_LON) / 2.0;
        let lat = (HEL_MIN_LAT + HEL_MAX_LAT) / 2.0;
        let valid = make_geojson(lon, lat);
        let outside = make_geojson(23.761, 61.498);

        write_file(&file, &format!("{valid}\nnot-json\n{outside}\n"));

        let (addresses, points) =
            load_addresses(file.to_str().expect("utf-8 path")).expect("expected successful load");
        assert_eq!(addresses.len(), 1);
        assert_eq!(points.len(), 1);
        assert_eq!(points[0].index, 0);
    }

    #[test]
    fn load_addresses_missing_file_returns_error() {
        let dir = unique_temp_dir("load_addresses_err");
        let missing = dir.join("missing.ndgeojson");

        assert!(load_addresses(missing.to_str().expect("utf-8 path")).is_err());
    }

    fn write_minimal_gtfs(dir: &Path) {
        write_file(
            &dir.join("stops.txt"),
            "stop_id,stop_lat,stop_lon\nS1,60.17,24.94\nS2,60.20,24.97\n",
        );
        write_file(
            &dir.join("routes.txt"),
            "route_id,route_short_name,route_type\nR1,10,3\nR2,200,700\n",
        );
        write_file(
            &dir.join("trips.txt"),
            "route_id,trip_id,shape_id\nR1,T1,SH1\nR2,T2,SH2\nR1,T4,SH1\nUNKNOWN,T3,SHX\n",
        );
        write_file(
            &dir.join("stop_times.txt"),
            "trip_id,stop_id\nT1,S1\nT1,S2\nT2,S2\nT9,S1\nT1,NO_STOP\n",
        );
        write_file(
            &dir.join("shapes.txt"),
            "shape_id,shape_pt_lat,shape_pt_lon,shape_pt_sequence\nSH1,60.21,24.91,2\nSH1,60.19,24.89,1\nSH2,60.22,24.92,5\nSH3,60.25,24.95,1\n",
        );
    }

    #[test]
    fn build_transit_dataset_from_dir_builds_expected_structures() {
        let dir = unique_temp_dir("gtfs_ok");
        write_minimal_gtfs(&dir);

        let dataset = build_transit_dataset_from_dir(&dir).expect("expected dataset");

        assert_eq!(dataset.stops.len(), 2);
        assert_eq!(dataset.routes.len(), 2);
        assert_eq!(dataset.shapes.len(), 3);
        assert_eq!(dataset.stop_routes.len(), 2);

        assert_eq!(dataset.routes[0].id, 0);
        assert_eq!(dataset.routes[0].short_name, "10");
        assert_eq!(dataset.routes[0].route_type, 3);
        assert_eq!(dataset.routes[0].shapes, vec![0]);

        assert_eq!(dataset.routes[1].id, 1);
        assert_eq!(dataset.routes[1].short_name, "200");
        assert_eq!(dataset.routes[1].route_type, 700);
        assert_eq!(dataset.routes[1].shapes, vec![1]);

        assert_eq!(dataset.shapes[0].id, 0);
        assert_eq!(dataset.shapes[0].points.len(), 2);
        assert_eq!(dataset.shapes[1].id, 1);
        assert_eq!(dataset.shapes[2].id, 2);

        assert_eq!(dataset.stop_routes[0].stop_id, 0);
        assert_eq!(dataset.stop_routes[0].routes, vec![0]);
        assert_eq!(dataset.stop_routes[1].stop_id, 1);
        assert_eq!(dataset.stop_routes[1].routes, vec![0, 1]);
    }

    #[test]
    fn build_transit_dataset_clamps_route_type_to_u16_max() {
        let dir = unique_temp_dir("gtfs_route_type_clamp");
        write_file(
            &dir.join("stops.txt"),
            "stop_id,stop_lat,stop_lon\nS1,60.17,24.94\n",
        );
        write_file(
            &dir.join("routes.txt"),
            "route_id,route_short_name,route_type\nR1,10,999999\n",
        );
        write_file(
            &dir.join("trips.txt"),
            "route_id,trip_id,shape_id\nR1,T1,SH1\n",
        );
        write_file(&dir.join("stop_times.txt"), "trip_id,stop_id\nT1,S1\n");
        write_file(
            &dir.join("shapes.txt"),
            "shape_id,shape_pt_lat,shape_pt_lon,shape_pt_sequence\nSH1,60.2,24.9,1\n",
        );

        let dataset = build_transit_dataset_from_dir(&dir).expect("expected dataset");
        assert_eq!(dataset.routes.len(), 1);
        assert_eq!(dataset.routes[0].route_type, u16::MAX);
    }

    #[test]
    fn build_transit_dataset_returns_error_for_invalid_csv_row() {
        let dir = unique_temp_dir("gtfs_invalid_csv");
        write_file(
            &dir.join("stops.txt"),
            "stop_id,stop_lat,stop_lon\nS1,NOT_A_FLOAT,24.94\n",
        );

        let result = build_transit_dataset_from_dir(&dir);
        assert!(result.is_err());
    }

    #[test]
    fn write_transit_dataset_writes_roundtrippable_output() {
        let dir = unique_temp_dir("write_dataset_ok");
        let out = dir.join("routes.bin.zst");

        let dataset = TransitDataset {
            stops: vec![(1, 2, 3)],
            routes: vec![Route {
                id: 1,
                short_name: "10".to_string(),
                route_type: 3,
                shapes: vec![7],
            }],
            shapes: vec![Shape {
                id: 7,
                points: vec![(4, 5)],
            }],
            stop_routes: vec![StopRoutes {
                stop_id: 1,
                routes: vec![1],
            }],
        };

        write_transit_dataset(&dataset, &out).expect("write should succeed");
        let bytes = fs::read(&out).expect("read output");
        assert!(!bytes.is_empty());

        let decoded = zstd::decode_all(&bytes[..]).expect("zstd decode");
        let restored: TransitDataset = bitcode::deserialize(&decoded).expect("bitcode decode");
        assert_eq!(restored.stops.len(), 1);
        assert_eq!(restored.routes.len(), 1);
        assert_eq!(restored.shapes.len(), 1);
        assert_eq!(restored.stop_routes.len(), 1);
        assert_eq!(restored.routes[0].short_name, "10");
    }

    #[test]
    fn write_transit_dataset_returns_error_when_output_path_is_directory() {
        let dir = unique_temp_dir("write_dataset_err");
        let output_is_directory = dir.join("outdir");
        fs::create_dir_all(&output_is_directory).expect("create dir");

        let dataset = TransitDataset {
            stops: Vec::new(),
            routes: Vec::new(),
            shapes: Vec::new(),
            stop_routes: Vec::new(),
        };

        let result = write_transit_dataset(&dataset, &output_is_directory);
        assert!(result.is_err());
    }

    #[test]
    fn resolve_default_hsl_input_dir_prefers_direct_data_dir_when_present() {
        let _guard = cwd_lock().lock().expect("lock cwd");
        let original = std::env::current_dir().expect("current dir");

        let dir = unique_temp_dir("resolve_default_direct");
        let direct = dir.join("data").join("hsl-GTFS-2026-03-12");
        fs::create_dir_all(&direct).expect("create direct data dir");

        std::env::set_current_dir(&dir).expect("set cwd");
        let result = resolve_default_hsl_input_dir();
        std::env::set_current_dir(original).expect("restore cwd");

        assert_eq!(result, PathBuf::from("data/hsl-GTFS-2026-03-12"));
    }

    #[test]
    fn resolve_default_hsl_input_dir_falls_back_to_parent_data_dir() {
        let _guard = cwd_lock().lock().expect("lock cwd");
        let original = std::env::current_dir().expect("current dir");

        let dir = unique_temp_dir("resolve_default_fallback");

        std::env::set_current_dir(&dir).expect("set cwd");
        let result = resolve_default_hsl_input_dir();
        std::env::set_current_dir(original).expect("restore cwd");

        assert_eq!(result, PathBuf::from("../data/hsl-GTFS-2026-03-12"));
    }
}
