use std::path::Path;

#[derive(Default)]
struct DatasetMetadata {
    publisher: String,
    start_date: String,
    end_date: String,
    version: String,
    source_path: String,
}

fn find_feed_info_files(dir: &Path, out: &mut Vec<String>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            find_feed_info_files(&path, out);
            continue;
        }

        if path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.eq_ignore_ascii_case("feed_info.txt"))
        {
            out.push(path.to_string_lossy().to_string());
        }
    }
}

fn load_dataset_metadata() -> DatasetMetadata {
    let mut files = Vec::new();
    find_feed_info_files(Path::new("../data"), &mut files);
    files.sort();

    let Some(path) = files.last() else {
        return DatasetMetadata::default();
    };

    println!("cargo:rerun-if-changed={}", path);

    let Ok(contents) = std::fs::read_to_string(path) else {
        return DatasetMetadata::default();
    };

    let mut lines = contents.lines();
    let _header = lines.next();
    let Some(values_line) = lines.next() else {
        return DatasetMetadata::default();
    };

    let fields: Vec<String> = values_line
        .split(',')
        .map(|value| value.trim().to_string())
        .collect();

    if fields.len() < 6 {
        return DatasetMetadata::default();
    }

    DatasetMetadata {
        publisher: fields[0].clone(),
        start_date: fields[3].clone(),
        end_date: fields[4].clone(),
        version: fields[5].clone(),
        source_path: path.clone(),
    }
}

fn emit_dataset_metadata_env() {
    let metadata = load_dataset_metadata();

    println!(
        "cargo:rustc-env=HSL_DATASET_PUBLISHER={}",
        metadata.publisher
    );
    println!(
        "cargo:rustc-env=HSL_DATASET_START_DATE={}",
        metadata.start_date
    );
    println!("cargo:rustc-env=HSL_DATASET_END_DATE={}", metadata.end_date);
    println!("cargo:rustc-env=HSL_DATASET_VERSION={}", metadata.version);
    println!(
        "cargo:rustc-env=HSL_DATASET_SOURCE_PATH={}",
        metadata.source_path
    );
}

fn main() {
    // Skip data file check if SKIP_DATA_CHECK env var is set
    // or if we're doing a secondary build (like for binaries)
    if std::env::var("SKIP_DATA_CHECK").is_ok() {
        emit_dataset_metadata_env();
        tauri_build::build();
        return;
    }

    // Check if the required data file exists
    let data_file = "../data/helsinki.bin.zst";
    if !Path::new(data_file).exists() {
        eprintln!("\n❌ ERROR: Required data file 'data/helsinki.bin.zst' not found!\n");
        eprintln!("This file needs to be generated before building the application.");
        eprintln!("\nTo generate it, run:");
        eprintln!(
            "  cd assignment-hsl-routes-react && cargo run --release --package converter --bin lookup\n"
        );
        eprintln!("Make sure 'data/finland.ndgeojson' exists before running this command.\n");
        panic!("Missing required data file: data/helsinki.bin.zst");
    }

    emit_dataset_metadata_env();

    tauri_build::build()
}
