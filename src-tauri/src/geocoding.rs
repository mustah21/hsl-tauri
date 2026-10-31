//! Address geocoding functionality for forward and reverse geocoding.
//!
//! This module handles:
//! - Loading compressed binary geocoding data
//! - Reverse geocoding (coordinates → address)
//! - Forward geocoding (address text → coordinates)
//! - Coordinate dequantization (u16 → f64)

use crate::AppError;
use converter::{Address, AddressPoint};
use rstar::RTree;
use std::sync::Arc;

// Bounding box constants for Helsinki area (must match converter)
const MIN_LON: f64 = 24.7968139972634;
const MAX_LON: f64 = 25.129150423044536;
const MIN_LAT: f64 = 60.132332639707755;
const MAX_LAT: f64 = 60.25384126877702;

/// State container for geocoding data.
///
/// Contains the R-tree spatial index and address list loaded from compressed binary.
pub struct GeocodingData {
    pub rtree: RTree<AddressPoint>,
    pub addresses: Vec<Address>,
}

impl GeocodingData {
    /// Loads geocoding data from zstd-compressed bitcode bytes.
    pub fn from_bytes(zstd_bytes: &[u8]) -> Result<Self, AppError> {
        let decompressed = zstd::decode_all(zstd_bytes)
            .map_err(|e| AppError::Serialization(format!("Failed to decompress data: {}", e)))?;


        Ok(GeocodingData { rtree, addresses })
    }

    /// Loads geocoding data from compressed binary file.
    ///
    /// Attempts to read from `../data/helsinki.bin.zst` or `data/helsinki.bin.zst`,
    /// decompresses with zstd, and deserializes with bitcode.
    pub fn load() -> Result<Self, AppError> {
        todo!("Implement binary loading: read data/helsinki.bin.zst, decompress with zstd, deserialize with bitcode")
    }
}

/// Dequantizes a u16 coordinate back to f64 within a given range.
///
/// Reverses the quantization performed by the converter.
///
/// # Examples
///
/// ```
/// use hsl_routes_lib::dequantize;
///
/// // Dequantize a midpoint value (32767 / 65535 ≈ 0.5)
/// let value = dequantize(32767, 10.0, 20.0);
/// assert!((value - 15.0).abs() < 0.01);
///
/// // Boundary values
/// assert_eq!(dequantize(0, 0.0, 1.0), 0.0);
/// assert!((dequantize(65535, 0.0, 1.0) - 1.0).abs() < 1e-9);
/// ```
pub fn dequantize(quantized: u16, min: f64, max: f64) -> f64 {
    let normalized = quantized as f64 / 65535.0;
    min + (normalized * (max - min))
}

/// Converts quantized address coordinates to f64.
///
/// Returns (latitude, longitude) as f64 tuple.
fn address_to_coords(addr: &Address) -> (f64, f64) {
    let lat = dequantize(addr.lat, MIN_LAT, MAX_LAT);
    (lat, lon)
}

/// Formats an address as a display string.
///
/// Format: "Street Number, Postcode City"
fn format_address(addr: &Address) -> String {
    format!(
        "{} {}, {} {}",
        addr.street, addr.number, addr.postcode, addr.city
    )
}

fn exact_full_address_match(addr_str: &str, query_lower: &str) -> Option<u32> {
    (addr_str == query_lower).then_some(1000)
}

fn exact_street_and_number_match(street_str: &str, query_lower: &str) -> Option<u32> {
    (street_str == query_lower).then_some(900)
}

fn prefix_match_score(addr_str: &str, street_str: &str, query_lower: &str) -> Option<u32> {
    if addr_str.starts_with(query_lower) {
        return Some(800);
    }
    if street_str.starts_with(query_lower) {
        return Some(700);
    }

    None
}

fn street_only_match_score(street_only: &str, query_lower: &str) -> Option<u32> {
    if street_only == query_lower {
        return Some(600);
    }
    if street_only.starts_with(query_lower) {
        return Some(500);
    }

    None
}

fn word_prefix_match_score(addr_str: &str, street_only: &str, query_lower: &str) -> Option<u32> {
    if !(addr_str.contains(query_lower) && query_lower.len() > 3) {
        return None;
    }

    query_lower
        .split_whitespace()
        .filter(|qword| qword.len() >= 3)
        .any(|qword| {
            street_only
                .split_whitespace()
                .any(|sword| sword.starts_with(qword))
        })
        .then_some(400)
}

pub(crate) fn reverse_geocode_inner(
    lat: f64,
    lng: f64,
    data: &GeocodingData,
) -> Result<String, AppError> {
    let query_point = AddressPoint {
        point: [lng as f32, lat as f32],
        index: 0,
    };

    let nearest = data
        .rtree
        .nearest_neighbor(&query_point)
        .ok_or_else(|| AppError::NotFound("No nearby address found".to_string()))?;

    let addr = &data.addresses[nearest.index as usize];

    Ok(format_address(addr))
}

/// Tauri command for reverse geocoding (coordinates → address).
///
/// Finds the nearest address to the given coordinates using R-tree spatial index.
#[cfg(not(tarpaulin_include))]
#[tauri::command]
pub fn reverse_geocode(
    lat: f64,
    lng: f64,
    geocoding_data: tauri::State<Arc<GeocodingData>>,
) -> Result<String, AppError> {
    reverse_geocode_inner(lat, lng, &geocoding_data)
}

/// Calculates a match score for an address against a search query.
///
/// Returns a score from 0-1000 where higher is better:
/// - 1000: Exact full address match
/// - 900: Street + number exact match
/// - 800: Address starts with query
/// - 700: Street + number starts with query
/// - 600: Street name exact match
/// - 500: Street name starts with query
/// - 400: Address contains query with street name word match
/// - 0: No match
fn calculate_address_match_score(addr: &Address, query_lower: &str) -> u32 {
    let addr_str = format!(
        "{} {}, {} {}",
        addr.street, addr.number, addr.postcode, addr.city
    )
    .to_lowercase();
    let street_str = format!("{} {}", addr.street, addr.number).to_lowercase();
    let street_only = addr.street.to_lowercase();

    exact_full_address_match(&addr_str, query_lower)
        .or_else(|| exact_street_and_number_match(&street_str, query_lower))
        .or_else(|| prefix_match_score(&addr_str, &street_str, query_lower))
        .or_else(|| street_only_match_score(&street_only, query_lower))
        .or_else(|| word_prefix_match_score(&addr_str, &street_only, query_lower))
        .unwrap_or(0)
}

/// Finds the best matching address for a query string.
///
/// Returns the address with the highest match score if score >= 400.
fn find_best_matching_address<'a>(addresses: &'a [Address], query: &str) -> Option<&'a Address> {
    let query_lower = query.trim().to_lowercase();
    if query_lower.is_empty() {
        return None;
    }

    addresses
        .iter()
        .map(|addr| (addr, calculate_address_match_score(addr, &query_lower)))
        .max_by_key(|(_, score)| *score)
        .filter(|(_, score)| *score >= 400)
        .map(|(addr, _)| addr)
}

pub(crate) fn forward_geocode_internal(
    query: &str,
    addresses: &[Address],
) -> Result<(f64, f64, String), AppError> {
    let addr = find_best_matching_address(addresses, query)
        .ok_or_else(|| AppError::NotFound("No matching address found".to_string()))?;
    let (lat, lon) = address_to_coords(addr);
    Ok((lat, lon, format_address(addr)))
}

/// Tauri command for forward geocoding (address text → coordinates).
///
/// Searches for the best matching address and returns its coordinates.
/// Returns tuple of (latitude, longitude, formatted_address).
#[cfg(not(tarpaulin_include))]
#[tauri::command]
pub fn forward_geocode(
    query: String,
    geocoding_data: tauri::State<Arc<GeocodingData>>,
) -> Option<(f64, f64, String)> {
    forward_geocode_internal(&query, &geocoding_data.addresses).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_address() -> Address {
        Address {
            street: "Mannerheimintie".to_string(),
            number: "1".to_string(),
            city: "Helsinki".to_string(),
            postcode: "00100".to_string(),
            lat: 32767,
            lon: 32767,
        }
    }

    #[test]
    fn test_format_address() {
        let addr = sample_address();
        assert_eq!(format_address(&addr), "Mannerheimintie 1, 00100 Helsinki");
    }

    #[test]
    fn test_address_to_coords_in_range() {
        let addr = sample_address(); // lat/lon = 32767 ≈ mid-range
        let (lat, lon) = address_to_coords(&addr);
        assert!(lat > MIN_LAT && lat < MAX_LAT);
        assert!(lon > MIN_LON && lon < MAX_LON);
    }

    #[test]
    fn test_address_to_coords_min_values() {
        let addr = Address {
            street: "Test".to_string(),
            number: "1".to_string(),
            city: "Helsinki".to_string(),
            postcode: "00100".to_string(),
            lat: 0,
            lon: 0,
        };
        let (lat, lon) = address_to_coords(&addr);
        assert!((lat - MIN_LAT).abs() < 1e-9);
        assert!((lon - MIN_LON).abs() < 1e-9);
    }

    #[test]
    fn test_exact_full_address_match_hit() {
        assert_eq!(
            exact_full_address_match(
                "mannerheimintie 1, 00100 helsinki",
                "mannerheimintie 1, 00100 helsinki"
            ),
            Some(1000)
        );
    }

    #[test]
    fn test_exact_full_address_match_miss() {
        assert_eq!(
            exact_full_address_match("mannerheimintie 1, 00100 helsinki", "other street"),
            None
        );
    }

    #[test]
    fn test_exact_street_and_number_match_hit() {
        assert_eq!(
            exact_street_and_number_match("mannerheimintie 1", "mannerheimintie 1"),
            Some(900)
        );
    }

    #[test]
    fn test_exact_street_and_number_match_miss() {
        assert_eq!(
            exact_street_and_number_match("mannerheimintie 1", "other"),
            None
        );
    }

    #[test]
    fn test_prefix_match_score_addr_starts_with() {
        // addr_str starts with query → 800
        let score = prefix_match_score(
            "mannerheimintie 1, 00100 helsinki",
            "mannerheimintie 1",
            "mannerheimintie 1,",
        );
        assert_eq!(score, Some(800));
    }

    #[test]
    fn test_prefix_match_score_street_starts_with() {
        // addr_str does NOT start with query, but street_str does → 700
        let score = prefix_match_score(
            "annankatu 10, 00120 helsinki",
            "mannerheimintie 1",
            "mannerheimintie",
        );
        assert_eq!(score, Some(700));
    }

    #[test]
    fn test_prefix_match_score_none() {
        let score = prefix_match_score("annankatu 10, 00120 helsinki", "annankatu 10", "bulevardi");
        assert_eq!(score, None);
    }

    #[test]
    fn test_street_only_match_score_exact() {
        assert_eq!(
            street_only_match_score("mannerheimintie", "mannerheimintie"),
            Some(600)
        );
    }

    #[test]
    fn test_street_only_match_score_prefix() {
        assert_eq!(
            street_only_match_score("mannerheimintie", "manner"),
            Some(500)
        );
    }

    #[test]
    fn test_street_only_match_score_none() {
        assert_eq!(
            street_only_match_score("mannerheimintie", "bulevardi"),
            None
        );
    }

    #[test]
    fn test_word_prefix_match_score_hit() {
        // query contained in addr, length > 3, and a word in query starts a word in street
        let score = word_prefix_match_score(
            "mannerheimintie 1, 00100 helsinki",
            "mannerheimintie",
            "mann",
        );
        assert_eq!(score, Some(400));
    }

    #[test]
    fn test_word_prefix_match_score_short_query() {
        // query length <= 3 → always None
        let score = word_prefix_match_score(
            "mannerheimintie 1, 00100 helsinki",
            "mannerheimintie",
            "man",
        );
        assert_eq!(score, None);
    }

    #[test]
    fn test_word_prefix_match_score_not_contained() {
        // query not present in addr_str → None
        let score = word_prefix_match_score(
            "mannerheimintie 1, 00100 helsinki",
            "mannerheimintie",
            "bulevardi",
        );
        assert_eq!(score, None);
    }

    #[test]
    fn test_calculate_address_match_score_exact() {
        let address = sample_address();
        let score = calculate_address_match_score(&address, "mannerheimintie 1, 00100 helsinki");
        assert_eq!(score, 1000);
    }

    #[test]
    fn test_calculate_address_match_score_street_and_number() {
        let address = sample_address();
        let score = calculate_address_match_score(&address, "mannerheimintie 1");
        assert_eq!(score, 900);
    }

    #[test]
    fn test_calculate_address_match_score_street_only() {
        let address = sample_address();
        let score = calculate_address_match_score(&address, "mannerheimintie");
        // addr_str starts with "mannerheimintie" → 800
        assert_eq!(score, 800);
    }

    #[test]
    fn test_calculate_address_match_score_street_name_exact() {
        let address = Address {
            street: "Mannerheimintie".to_string(),
            number: "1".to_string(),
            city: "Helsinki".to_string(),
            postcode: "00100".to_string(),
            lat: 32767,
            lon: 32767,
        };
        // Score 600: street_only == query_lower, but full addr and street+num don't match
        // Use a query equal to street name only (it would hit 800 via addr prefix first)
        // so use a query "mannerheimintie 1, 00100 helsinki_x" to test 600 path indirectly
        // Instead test via street_only_match_score directly
        let score = street_only_match_score("mannerheimintie", "mannerheimintie");
        assert_eq!(score, Some(600));
        drop(address);
    }

    #[test]
    fn test_calculate_address_match_score_no_match() {
        let address = sample_address();
        let score = calculate_address_match_score(&address, "bulevardi");
        assert_eq!(score, 0);
    }

    #[test]
    fn test_find_best_matching_address_none_for_empty_query() {
        let addresses = vec![sample_address()];
        let result = find_best_matching_address(&addresses, "   ");
        assert!(result.is_none());
    }

    #[test]
    fn test_find_best_matching_address_success() {
        let addresses = vec![sample_address()];
        let result = find_best_matching_address(&addresses, "Mannerheimintie");
        assert!(result.is_some());
        assert_eq!(result.unwrap().street, "Mannerheimintie");
    }

    #[test]
    fn test_find_best_matching_address_below_threshold() {
        // Query that gives score 0 → None
        let addresses = vec![sample_address()];
        let result = find_best_matching_address(&addresses, "completely unrelated xyz");
        assert!(result.is_none());
    }

    #[test]
    fn test_find_best_matching_address_empty_list() {
        let result = find_best_matching_address(&[], "Mannerheimintie");
        assert!(result.is_none());
    }

    #[test]
    fn test_forward_geocode_internal_success() {
        let addresses = vec![sample_address()];
        let result = forward_geocode_internal("Mannerheimintie", &addresses);
        assert!(result.is_ok());
        let (lat, lon, formatted) = result.unwrap();
        assert!(lat > MIN_LAT && lat < MAX_LAT);
        assert!(lon > MIN_LON && lon < MAX_LON);
        assert!(formatted.contains("Mannerheimintie"));
    }

    #[test]
    fn test_forward_geocode_internal_returns_not_found() {
        let result = forward_geocode_internal("unknown street", &[sample_address()]);
        assert!(matches!(result, Err(AppError::NotFound(_))));
    }

    // --- reverse_geocode_inner ---

    fn make_geocoding_data() -> GeocodingData {
        use converter::AddressPoint;
        use rstar::RTree;
        let addr = sample_address();
        let point = AddressPoint {
            point: [
                dequantize(addr.lon, MIN_LON, MAX_LON) as f32,
                dequantize(addr.lat, MIN_LAT, MAX_LAT) as f32,
            ],
            index: 0,
        };
        let rtree = RTree::bulk_load(vec![point]);
        GeocodingData {
            rtree,
            addresses: vec![addr],
        }
    }

    #[test]
    fn test_reverse_geocode_inner_success() {
        let data = make_geocoding_data();
        // Use the same coordinates as the sample address
        let lat = dequantize(32767, MIN_LAT, MAX_LAT);
        let lon = dequantize(32767, MIN_LON, MAX_LON);
        let result = reverse_geocode_inner(lat, lon, &data);
        assert!(result.is_ok());
        assert!(result.unwrap().contains("Mannerheimintie"));
    }

    #[test]
    fn test_reverse_geocode_inner_empty_rtree_returns_not_found() {
        use rstar::RTree;
        let data = GeocodingData {
            rtree: RTree::new(),
            addresses: vec![],
        };
        let result = reverse_geocode_inner(60.17, 24.94, &data);
        assert!(matches!(result, Err(AppError::NotFound(_))));
    }

    // --- from_bytes ---

    fn make_geocoding_bytes() -> Vec<u8> {
        use converter::AddressPoint;
        use rstar::RTree;
        let addr = sample_address();
        let point = AddressPoint {
            point: [
                dequantize(addr.lon, MIN_LON, MAX_LON) as f32,
                dequantize(addr.lat, MIN_LAT, MAX_LAT) as f32,
            ],
            index: 0,
        };
        let rtree: RTree<AddressPoint> = RTree::bulk_load(vec![point]);
        let encoded = bitcode::serialize(&(rtree, vec![addr])).unwrap();
        zstd::encode_all(&encoded[..], 1).unwrap()
    }

    #[test]
    fn test_from_bytes_success() {
        let bytes = make_geocoding_bytes();
        let result = GeocodingData::from_bytes(&bytes);
        assert!(result.is_ok());
        let data = result.unwrap();
        assert_eq!(data.addresses.len(), 1);
    }

    #[test]
    fn test_from_bytes_invalid_zstd() {
        let result = GeocodingData::from_bytes(b"not valid zstd data at all xyz");
        assert!(matches!(result, Err(AppError::Serialization(_))));
    }

    #[test]
    fn test_from_bytes_invalid_bitcode() {
        // Valid zstd but garbage bitcode inside
        let garbled = zstd::encode_all(b"garbage_bitcode" as &[u8], 1).unwrap();
        let result = GeocodingData::from_bytes(&garbled);
        assert!(matches!(result, Err(AppError::Serialization(_))));
    }

    #[test]
    fn test_load_missing_file_returns_io_error() {
        let lock = crate::routes::shared_cwd_lock().lock().unwrap();

        let temp_dir = std::env::temp_dir().join("geocoding_load_miss");
        let _ = std::fs::remove_dir_all(&temp_dir);
        std::fs::create_dir_all(&temp_dir).unwrap();
        let orig = std::env::current_dir().unwrap();
        std::env::set_current_dir(&temp_dir).unwrap();

        let result = GeocodingData::load();

        std::env::set_current_dir(&orig).unwrap();
        let _ = std::fs::remove_dir_all(&temp_dir);
        drop(lock);

        assert!(matches!(result, Err(AppError::Io(_))));
    }

    #[test]
    fn test_load_from_file_success() {
        let lock = crate::routes::shared_cwd_lock().lock().unwrap();

        let temp_dir = std::env::temp_dir().join("geocoding_load_ok");
        let _ = std::fs::remove_dir_all(&temp_dir);
        let data_dir = temp_dir.join("data");
        std::fs::create_dir_all(&data_dir).unwrap();

        let bytes = make_geocoding_bytes();
        std::fs::write(data_dir.join("helsinki.bin.zst"), &bytes).unwrap();

        let orig = std::env::current_dir().unwrap();
        std::env::set_current_dir(&temp_dir).unwrap();

        let result = GeocodingData::load();

        std::env::set_current_dir(&orig).unwrap();
        let _ = std::fs::remove_dir_all(&temp_dir);
        drop(lock);

        assert!(result.is_ok());
    }

    #[test]
    fn test_address_to_coords_max_values() {
        let addr = Address {
            street: "Test".to_string(),
            number: "1".to_string(),
            city: "Helsinki".to_string(),
            postcode: "00100".to_string(),
            lat: 65535,
            lon: 65535,
        };
        let (lat, lon) = address_to_coords(&addr);
        assert!((lat - MAX_LAT).abs() < 1e-9);
        assert!((lon - MAX_LON).abs() < 1e-9);
    }

    #[test]
    fn test_find_best_matching_address_selects_best_of_multiple() {
        // Two addresses: only one matches the query well enough (score >= 400)
        let addr_bulevardi = Address {
            street: "Bulevardi".to_string(),
            number: "5".to_string(),
            city: "Helsinki".to_string(),
            postcode: "00120".to_string(),
            lat: 30000,
            lon: 30000,
        };
        let addr_mannerheim = sample_address(); // "Mannerheimintie 1"
        let addresses = vec![addr_bulevardi, addr_mannerheim];

        // Exact street+number match gives score 900 for Mannerheimintie 1
        let result = find_best_matching_address(&addresses, "Mannerheimintie 1");
        assert!(result.is_some());
        assert_eq!(result.unwrap().street, "Mannerheimintie");
    }

    #[test]
    fn test_calculate_address_match_score_word_prefix_path() {
        // A multi-word street name allows a query that is a suffix-word of the street to
        // reach the word_prefix_match_score branch (score 400) without triggering any
        // higher-scoring match first.
        //
        // addr_str = "iso roobertinkatu 10, 00120 helsinki"
        // query "roobert" (len=7 > 3):
        //   - not equal to addr_str   → excludes 1000
        //   - not equal to street_str → excludes 900
        //   - addr_str.starts_with("roobert") = false → excludes 800
        //   - street_str.starts_with("roobert") = false → excludes 700
        //   - street_only ("iso roobertinkatu") != "roobert" → excludes 600
        //   - street_only.starts_with("roobert") = false → excludes 500
        //   - addr_str.contains("roobert") = true, len > 3
        //     word "roobert" is a prefix of "roobertinkatu" in street_only → 400
        let addr = Address {
            street: "Iso Roobertinkatu".to_string(),
            number: "10".to_string(),
            city: "Helsinki".to_string(),
            postcode: "00120".to_string(),
            lat: 32767,
            lon: 32767,
        };
        let score = calculate_address_match_score(&addr, "roobert");
        assert_eq!(score, 400);
    }
}
