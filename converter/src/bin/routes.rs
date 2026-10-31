use anyhow::Result;
use converter::{build_transit_dataset_from_dir, write_transit_dataset};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

#[cfg(not(test))]
use converter::resolve_default_hsl_input_dir;

fn route_type_label(route_type: u16) -> &'static str {
    match route_type {
        0 => "Tram",
        1 => "Subway",
        2 => "Rail",
        3 => "Bus",
        4 => "Ferry",
        5 => "Cable tram",
        6 => "Aerial lift",
        7 => "Funicular",
        11 => "Trolleybus",
        12 => "Monorail",
        109 => "Suburban rail",
        700 => "Coach",
        701 => "Regional bus",
        702 => "Express bus",
        704 => "Local bus",
        900 => "Tram service",
        _ => "Unknown",
    }
}

fn route_type_counts(dataset: &converter::TransitDataset) -> BTreeMap<u16, usize> {
    let mut route_type_counts: BTreeMap<u16, usize> = BTreeMap::new();
    for route in &dataset.routes {
        *route_type_counts.entry(route.route_type).or_insert(0) += 1;
    }
    route_type_counts
}

fn build_and_write_routes(
    input_dir: &Path,
    output_path: &Path,
) -> Result<converter::TransitDataset> {
    let dataset = build_transit_dataset_from_dir(input_dir)?;
    write_transit_dataset(&dataset, output_path)?;
    Ok(dataset)
}

#[cfg(not(test))]
fn main() -> Result<()> {
    let mut args = std::env::args().skip(1);

    let input_dir = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(resolve_default_hsl_input_dir);

    let output_path = args.next().map(PathBuf::from).unwrap_or_else(|| {
        if input_dir.starts_with("../") {
            PathBuf::from("../data/routes.bin.zst")
        } else {
            PathBuf::from("data/routes.bin.zst")
        }
    });

    let dataset = build_and_write_routes(&input_dir, &output_path)?;

    println!("dataset built: {}", output_path.display());

    let route_type_counts = route_type_counts(&dataset);

    println!("\nRoute type counts:");
    println!("{:<10} {:<20} {:>8}", "type", "label", "count");
    println!("{:-<10} {:-<20} {:-<8}", "", "", "");
    for (route_type, count) in route_type_counts {
        println!(
            "{:<10} {:<20} {:>8}",
            route_type,
            route_type_label(route_type),
            count
        );
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn unique_temp_dir(prefix: &str) -> PathBuf {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time went backwards")
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("routes_{prefix}_{now}"));
        fs::create_dir_all(&dir).expect("failed to create temp dir");
        dir
    }

    fn write_file(path: &Path, content: &str) {
        fs::write(path, content).expect("failed to write file");
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
            "route_id,trip_id,shape_id\nR1,T1,SH1\nR2,T2,SH2\nR1,T4,SH1\n",
        );
        write_file(
            &dir.join("stop_times.txt"),
            "trip_id,stop_id\nT1,S1\nT1,S2\nT2,S2\n",
        );
        write_file(
            &dir.join("shapes.txt"),
            "shape_id,shape_pt_lat,shape_pt_lon,shape_pt_sequence\nSH1,60.19,24.89,1\nSH1,60.21,24.91,2\nSH2,60.22,24.92,1\n",
        );
    }

    #[test]
    fn route_type_label_covers_known_and_unknown_values() {
        assert_eq!(route_type_label(0), "Tram");
        assert_eq!(route_type_label(1), "Subway");
        assert_eq!(route_type_label(2), "Rail");
        assert_eq!(route_type_label(3), "Bus");
        assert_eq!(route_type_label(4), "Ferry");
        assert_eq!(route_type_label(5), "Cable tram");
        assert_eq!(route_type_label(6), "Aerial lift");
        assert_eq!(route_type_label(7), "Funicular");
        assert_eq!(route_type_label(11), "Trolleybus");
        assert_eq!(route_type_label(12), "Monorail");
        assert_eq!(route_type_label(109), "Suburban rail");
        assert_eq!(route_type_label(700), "Coach");
        assert_eq!(route_type_label(701), "Regional bus");
        assert_eq!(route_type_label(702), "Express bus");
        assert_eq!(route_type_label(704), "Local bus");
        assert_eq!(route_type_label(900), "Tram service");
        assert_eq!(route_type_label(999), "Unknown");
    }

    #[test]
    fn route_type_counts_aggregates_dataset_values() {
        let dataset = converter::TransitDataset {
            stops: Vec::new(),
            routes: vec![
                converter::Route {
                    id: 0,
                    short_name: "10".to_string(),
                    route_type: 3,
                    shapes: Vec::new(),
                },
                converter::Route {
                    id: 1,
                    short_name: "200".to_string(),
                    route_type: 700,
                    shapes: Vec::new(),
                },
                converter::Route {
                    id: 2,
                    short_name: "20".to_string(),
                    route_type: 3,
                    shapes: Vec::new(),
                },
            ],
            shapes: Vec::new(),
            stop_routes: Vec::new(),
        };

        let counts = route_type_counts(&dataset);
        assert_eq!(counts.get(&3), Some(&2));
        assert_eq!(counts.get(&700), Some(&1));
    }

    #[test]
    fn build_and_write_routes_writes_valid_output() {
        let dir = unique_temp_dir("ok");
        write_minimal_gtfs(&dir);
        let output = dir.join("routes.bin.zst");

        let dataset = build_and_write_routes(&dir, &output).expect("should build routes dataset");
        assert_eq!(dataset.routes.len(), 2);

        let bytes = fs::read(output).expect("read output file");
        assert!(!bytes.is_empty());
    }

    #[test]
    fn build_and_write_routes_returns_error_for_missing_input_files() {
        let dir = unique_temp_dir("missing");
        let output = dir.join("routes.bin.zst");

        assert!(build_and_write_routes(&dir, &output).is_err());
    }

    #[test]
    fn build_and_write_routes_returns_error_when_output_is_directory() {
        let dir = unique_temp_dir("output_dir");
        write_minimal_gtfs(&dir);
        let output_dir = dir.join("out");
        fs::create_dir_all(&output_dir).expect("create output dir");

        assert!(build_and_write_routes(&dir, &output_dir).is_err());
    }
}
