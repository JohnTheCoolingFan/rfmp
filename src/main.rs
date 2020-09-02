use std::{fs, env, io::copy};
use std::path::{Path, PathBuf};
use zip::write::{ZipWriter, FileOptions};
use walkdir::{DirEntry, WalkDir};
use dirs;
use glob;

fn main() {
    let mut check_old_versions = true;
    let mut next_path = false;
    let mut alternative_path = String::new() ;

    let mut args: Vec<String> = env::args().collect();
    args.remove(0);

    for arg in args {
        if next_path {
            alternative_path = arg;
        } else if arg == String::from("--install-dir") {
            next_path = true;
        } else if arg == String::from("--no-clean") {
            check_old_versions = false;
        }
    }

    // Open info.json and parse it
    let mut info_file = fs::File::open("info.json").unwrap();
    let info: serde_json::Value = serde_json::from_reader(&mut info_file).unwrap();

    // Get mod name/id and version
    let mod_name = info["name"].as_str().unwrap();
    let mod_version = info["version"].as_str().unwrap();
    
    if check_old_versions {
        // Check if any version of the mod already installed/exists.
        let mod_glob_str = format!("{}/.factorio/mods/{}_*[0-9].*[0-9].*[0-9].zip", dirs::home_dir().unwrap().to_str().unwrap(), mod_name);
        let mod_glob = glob::glob(&mod_glob_str).unwrap().into_iter();

        // Delete if exists
        for entry in mod_glob {
            let  entry = entry.unwrap();
            println!("Removing {}", entry.to_str().unwrap());
            if entry.is_file() {
                fs::remove_file(&entry).unwrap();
            }
        }
    }

    // Mod file name
    let zip_file_name = format!("{}_{}.zip", mod_name, mod_version);

    // Walkdir iter, filtered
    let walkdir = WalkDir::new(".");
    let it = walkdir.into_iter().filter_entry(|e| !is_hidden(e, &zip_file_name));

    // Mod file path
    //let zip_file_path = PathBuf::from(&zip_file_name);
    let mut zip_file_path: PathBuf;
    if alternative_path.is_empty() {
        zip_file_path = dirs::home_dir().unwrap();
        zip_file_path.push(".factorio");
        zip_file_path.push("mods");
    } else {
        zip_file_path = PathBuf::from(alternative_path);
    }
    zip_file_path.push(&zip_file_name);

    // Delete existing file
    if zip_file_path.exists() {
        println!("{} exists, removing.", zip_file_path.to_str().unwrap());
        if zip_file_path.is_file() {
            fs::remove_file(&zip_file_path).unwrap();
        } else if zip_file_path.is_dir() {
            fs::remove_dir(&zip_file_path).unwrap();
        }
    }

    // Create mod file
    let zip_file = fs::File::create(zip_file_path).unwrap();

    // Archive options. Deflated is best combination of speed and compression (for zip)
    // It would be cool if Factorio allowed other compression formats, like zstd
    let zip_options = FileOptions::default();

    // Create writer
    let mut zipwriter = ZipWriter::new(zip_file);  

    // Let the zipping begin!
    for entry in it {
        let entry = entry.unwrap();
        let name = entry.path();
        name.strip_prefix(Path::new(".")).unwrap();
        let zipped_name = Path::new(&format!("{}_{}", mod_name, mod_version)).join(&name);

        if name.is_file() {
            //println!("adding file {:?}", name);
            zipwriter.start_file_from_path(&zipped_name, zip_options).unwrap();
            let mut f = fs::File::open(name).unwrap();

            copy(&mut f, &mut zipwriter).unwrap();
        } else if name.as_os_str().len() != 0 {
            //println!("adding dir  {:?}", name);
            zipwriter.add_directory_from_path(&zipped_name, zip_options).unwrap();
        }
    }

    // Finish writing
    zipwriter.finish().unwrap();
}

// Function to filter all files we don't want to add to archive
fn is_hidden(entry: &DirEntry, zip_file_name: &String) -> bool {
    entry.file_name().to_str().unwrap() == zip_file_name || (entry.file_name().to_str().unwrap() != "." &&  entry.file_name().to_str().unwrap().starts_with("."))
}
