use std::{
    fs::{self, File},
    io::BufWriter,
    path::{Path, PathBuf},
};

use clap::Parser;
use glob::glob;
use mtzip::ZipArchive;
use serde::Deserialize;
use serde_json::from_reader;
use walkdir::{DirEntry, WalkDir};

#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
struct CliArgs {
    #[clap(
        short,
        long,
        value_name = "PATH",
        help = "Install mod to <PATH> instead of default path",
        long_help = "Install mod to <PATH> instead of default path.\nDefault path is `$HOME/.factorio/mods` on linux and `{{FOLDERID_RoamingAppData}}\\Factorio\\mods`.\nTakes priority over $FACTORIO_HOME environment variable"
    )]
    install_dir: Option<PathBuf>,

    #[clap(
        short,
        long,
        help = "Do not search for other versions of the mod and do not try to remove them."
    )]
    no_clean: bool,

    #[clap(
        short,
        long,
        value_name = "PATH",
        help = "Exclude files or directories from being included in the archive"
    )]
    exclude: Vec<PathBuf>,
}

#[derive(Deserialize)]
struct InfoJson {
    name: String,
    version: String,
}

fn main() {
    let CliArgs {
        install_dir,
        no_clean,
        exclude,
    } = CliArgs::parse();

    // Mods directory path
    let mut zip_file_path = install_dir.or_else(|| std::env::var("FACTORIO_HOME").map(PathBuf::from).ok()).unwrap_or_else(|| {
        if cfg!(target_os = "linux") {
            dirs::home_dir().unwrap().join(".factorio/mods")
        } else if cfg!(target_os = "windows") {
            dirs::data_dir().unwrap().join("Factorio/mods")
        } else {
            println!("Warning: unknown OS. Please report to github what OS you use and where `mods` directory is located. Using current directory as a fallback");
            PathBuf::from(".")
        }
    });

    if !zip_file_path.exists() {
        panic!("Error: {} doesn't exist", zip_file_path.to_string_lossy());
    }

    // Open info.json and parse it
    let info_file = File::open("info.json").expect("info.json not  found");
    let info_json: InfoJson = from_reader(info_file).expect("Failed to parse info.json");

    // Get mod name/id and version
    let mod_name_with_version = format!("{}_{}", info_json.name, info_json.version);

    // Check for other versions
    if !no_clean {
        // Check if any version of the mod already installed/exist.
        let mod_glob_str = format!(
            "{}/{}_*[0-9].*[0-9].*[0-9].zip",
            zip_file_path.to_string_lossy(),
            info_json.name
        );
        let mod_glob = glob(&mod_glob_str).unwrap();

        // Delete if any other versions found
        for entry in mod_glob {
            let entry = entry.unwrap();
            let entry_name = entry.to_string_lossy();
            println!("Removing {entry_name}");
            if entry.is_file() {
                fs::remove_file(&entry).unwrap();
            } else {
                println!("Failed to remove {entry_name}: not a file");
            }
        }
    }

    // Mod file name
    let zip_file_name = format!("{mod_name_with_version}.zip");
    zip_file_path.push(&zip_file_name);

    // As testing found out, removing the file beforehand speeds up the whole process
    // Delete existing file. This probably wouldn't run unless --no-clean argument is passed.
    if zip_file_path.exists() {
        println!("{} exists, removing.", zip_file_path.to_string_lossy());
        if zip_file_path.is_file() {
            fs::remove_file(&zip_file_path).unwrap();
        } else if zip_file_path.is_dir() {
            // Is this even possible?
            fs::remove_dir(&zip_file_path).unwrap();
        }
    }

    // Create archive
    let zipwriter = ZipArchive::default();

    // Add root dir
    //println!("Adding root dir");
    zipwriter.add_directory(mod_name_with_version.clone());

    let path_prefix = Path::new(&mod_name_with_version);

    // Walkdir iter, filtered
    let walkdir = WalkDir::new(".")
        .into_iter()
        .filter_entry(|e| !is_hidden(e, &zip_file_name, &exclude))
        .map(Result::unwrap)
        .map(|de| de.path().to_path_buf())
        .skip(1);

    // Let the zipping begin!
    for path in walkdir {
        let zip_path = path_prefix.join(path.strip_prefix("./").unwrap());
        let zipped_name = zip_path.to_string_lossy();

        if path.is_file() {
            //println!("adding file {:?}", zipped_name);
            zipwriter.add_file(path, &zipped_name);
        } else if !path.as_os_str().is_empty() {
            //println!("adding dir  {:?}", zipped_name);
            zipwriter.add_directory(zipped_name.to_string());
        }
    }

    // Create mod file
    let mut zip_file =
        BufWriter::new(File::create(zip_file_path).expect("Failed to open output file"));

    // Finish writing
    zipwriter.write(&mut zip_file);
}

// Function to filter all files we don't want to add to archive
fn is_hidden(entry: &DirEntry, zip_file_name: &str, excludes: &[PathBuf]) -> bool {
    let entry_file_name = entry.file_name().to_str().unwrap();
    entry_file_name == zip_file_name
        || (entry_file_name != "." && entry_file_name.starts_with('.'))
        || excludes.contains(&entry.path().into())
}
