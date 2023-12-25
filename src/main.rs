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
        help = "Do not search for other versions of the mod and do not try to remove them.",
        alias = "no_clean"
    )]
    keep_old_versions: bool,

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

fn get_default_factorio_home() -> PathBuf {
    if cfg!(target_os = "linux") {
        dirs::home_dir().unwrap().join(".factorio/mods")
    } else if cfg!(target_os = "windows") {
        dirs::data_dir().unwrap().join("Factorio/mods")
    } else {
        println!("Warning: unknown OS. Please report to github what OS you use and where `mods` directory is located. Using current directory as a fallback");
        PathBuf::from(".")
    }
}

/// Open info.json and parse it
fn get_info_json() -> InfoJson {
    let info_file = File::open("info.json").expect("info.json not  found");
    from_reader(info_file).expect("Failed to parse info.json")
}

fn main() {
    let CliArgs {
        install_dir,
        keep_old_versions,
        exclude,
    } = CliArgs::parse();

    // Mods directory path
    let mods_target_dir = install_dir
        .or_else(|| std::env::var("FACTORIO_HOME").map(PathBuf::from).ok())
        .unwrap_or_else(get_default_factorio_home);

    if !mods_target_dir.exists() {
        panic!("Error: {} doesn't exist", mods_target_dir.to_string_lossy());
    }

    let info_json = get_info_json();

    // Get mod name/id and version
    let mod_name_with_version = format!("{}_{}", info_json.name, info_json.version);

    // Check for other versions
    if !keep_old_versions {
        // Check if any version of the mod already installed/exist.
        let mod_glob_str = format!(
            "{}/{}_*[0-9].*[0-9].*[0-9].zip",
            mods_target_dir.to_string_lossy(),
            info_json.name
        );
        let mod_glob = glob(&mod_glob_str).expect("Failed to construct glob");

        // Delete if any other versions found
        for entry in mod_glob.filter_map(Result::ok) {
            println!("Removing {}", entry.to_string_lossy());
            if entry.is_file() {
                fs::remove_file(&entry).expect("Failed to remove file");
            } else {
                eprintln!("Failed to remove {}: not a file", entry.to_string_lossy());
            }
        }
    }

    // Mod file name
    let zip_file_name = format!("{mod_name_with_version}.zip");
    let target_zip_file = mods_target_dir.join(&zip_file_name);

    // As testing found out, removing the file beforehand speeds up the whole process
    // Delete existing file. This probably wouldn't run unless --no-clean argument is passed.
    if target_zip_file.exists() {
        println!("{} exists, removing.", target_zip_file.to_string_lossy());
        if target_zip_file.is_file() {
            fs::remove_file(&target_zip_file).expect("Failed to remove file");
        } else if target_zip_file.is_dir() {
            // Is this even possible?
            fs::remove_dir(&target_zip_file).expect("Failed to remove directory");
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
        .filter_map(|de_res| match de_res {
            Ok(de) => Some(de.path().to_path_buf()),
            Err(e) => {
                eprintln!("Error when walking the directory: {e}");
                None
            }
        })
        .skip(1);

    // Let the zipping begin!
    for path in walkdir {
        let zip_path = path_prefix.join(
            path.strip_prefix("./")
                .expect("Failed to strip './' prefix"),
        );
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
        BufWriter::new(File::create(target_zip_file).expect("Failed to open output file"));

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
