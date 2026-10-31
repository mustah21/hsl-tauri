use converter::{build_and_serialize_addresses, load_addresses};
use std::path::Path;

fn run_lookup_conversion(input_path: &Path, output_path: &Path) -> anyhow::Result<usize> {
    let (addresses, points) = load_addresses(input_path.to_str().expect("utf-8 path"))?;
    let address_count = addresses.len();
    let compressed = build_and_serialize_addresses(addresses, points)?;
    std::fs::write(output_path, compressed)?;
    Ok(address_count)
}

#[cfg(not(test))]
fn main() {
    let input = Path::new("data/finland.ndgeojson");
    let output = Path::new("data/helsinki.bin.zst");

    let count =
        run_lookup_conversion(input, output).expect("Failed to convert and write address data");

    println!("Loaded {} addresses within Helsinki bounds", count);
    println!("Successfully wrote {}", output.display());
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn unique_temp_dir(prefix: &str) -> std::path::PathBuf {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time went backwards")
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("lookup_{prefix}_{now}"));
        fs::create_dir_all(&dir).expect("failed to create temp dir");
        dir
    }

    fn make_geojson(lon: f64, lat: f64) -> String {
        format!(
            r#"{{"type":"Feature","geometry":{{"type":"Point","coordinates":[{lon},{lat}]}},"properties":{{"street":"Testikatu","number":"1","city":"Helsinki","postcode":"00100"}}}}"#
        )
    }

    #[test]
    fn run_lookup_conversion_writes_output_for_valid_input() {
        let dir = unique_temp_dir("ok");
        let input = dir.join("finland.ndgeojson");
        let output = dir.join("helsinki.bin.zst");

        let line = make_geojson(24.95, 60.19);
        fs::write(&input, format!("{line}\n")).expect("write input");

        let count = run_lookup_conversion(&input, &output).expect("conversion should succeed");
        assert_eq!(count, 1);
        let bytes = fs::read(output).expect("read output");
        assert!(!bytes.is_empty());
    }

    #[test]
    fn run_lookup_conversion_returns_error_for_missing_input() {
        let dir = unique_temp_dir("missing");
        let input = dir.join("no-file.ndgeojson");
        let output = dir.join("helsinki.bin.zst");

        assert!(run_lookup_conversion(&input, &output).is_err());
    }

    #[test]
    fn run_lookup_conversion_returns_error_when_output_is_directory() {
        let dir = unique_temp_dir("output_dir");
        let input = dir.join("finland.ndgeojson");
        let output_dir = dir.join("out");
        fs::create_dir_all(&output_dir).expect("create output dir");

        let line = make_geojson(24.95, 60.19);
        fs::write(&input, format!("{line}\n")).expect("write input");

        assert!(run_lookup_conversion(&input, &output_dir).is_err());
    }
}
