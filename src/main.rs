use std::{fs, env, io::copy};
use std::path::{Path, PathBuf};
use zip::write::{ZipWriter, FileOptions};
use serde_json::{from_reader, Value};
use walkdir::{DirEntry, WalkDir};
use dirs;
use glob;

fn print_help(executable_name: &String, exit_code: i32) {
    println!("Usage: {} [--install-dir PATH] [--no-clean]\n\n    \
                            No arguments: Pack mod from mod files in current path (pwd) and install into default mod path.\n\n    \
                            --install-dir PATH: Install mod to PATH instead of default one.\n      \
                            Default path is (on linux) ~/.factorio/mods\n\n    \
                            --no-clean: Do not search for other versions of the mod and do not remove them.\n\n    \
                            --help: Show this message.", executable_name);
    std::process::exit(exit_code);
}

fn main() {
    // Flags for args
    let mut check_old_versions = true;
    let mut next_path = false;
    let mut zip_file_path: PathBuf;

    // Mod file path
    #[cfg(target_os="linux")]
    {
        zip_file_path = dirs::home_dir().unwrap();
        zip_file_path.push(".factorio");
        zip_file_path.push("mods");
    }
    #[cfg(target_os="windows")]
    {
        zip_file_path = dirs::data_dir().unwrap();
        zip_file_path.push("Factorio");
        zip_file_path.push("mods");
    }

    let mut args: Vec<String> = env::args().collect();

    if args.len() != 1 {
        // This requires more reliability, especially user input checking.
        let executable_name = args.remove(0);
        for arg in args {
            // This part looks especially jank
            if next_path {
                zip_file_path = PathBuf::from(arg);
                next_path = false;
            } else {
                match arg.as_str() {
                    "--help" => print_help(&executable_name, 0),
                    "--install-dir" => next_path = true,
                    "--no-clean" => check_old_versions = false,
                    _ => print_help(&executable_name, 1),
                }
            }
        }
    }

    if !zip_file_path.exists() {
        panic!("Error: {:?} doesn't exist", zip_file_path);
    }

    // Open info.json and parse it
    let info_file = fs::File::open("info.json").expect("Error opening info.json");
    let info: Value = from_reader(info_file).expect("Error parsing info.json");

    // Get mod name/id and version
    let mod_name = info["name"].as_str().unwrap();
    let mod_version = info["version"].as_str().unwrap();
    
    if check_old_versions {
        // Check if any version of the mod already installed/exists.
        let mod_glob_str = format!("{}/{}_*[0-9].*[0-9].*[0-9].zip", zip_file_path.as_os_str().to_str().unwrap(), mod_name);
        let mod_glob = glob::glob(&mod_glob_str).unwrap().into_iter();

        // Delete if exists
        for entry in mod_glob {
            let entry = entry.unwrap();
            let entry_name = entry.to_str().unwrap();
            println!("Removing {}", entry_name);
            if entry.is_file() {
                fs::remove_file(&entry).unwrap();
            } else {
                println!("Failed to remove {}: not a file", entry_name);
            }
        }
    }

    // Mod file name
    let zip_file_name = format!("{}_{}.zip", mod_name, mod_version);
    zip_file_path.push(&zip_file_name);

    // Walkdir iter, filtered
    let walkdir = WalkDir::new(".");
    let it = walkdir.into_iter().filter_entry(|e| !is_hidden(e, &zip_file_name));

    // Delete existing file
    if zip_file_path.exists() {
        println!("{} exists, removing.", zip_file_path.to_str().unwrap());
        if zip_file_path.is_file() {
            fs::remove_file(&zip_file_path).unwrap();
        } else if zip_file_path.is_dir() { // Is this even possible?
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
    let entry_file_name = entry.file_name().to_str().unwrap();
    entry_file_name == zip_file_name ||
        (entry_file_name != "." && entry_file_name.starts_with(".")) ||
        entry_file_name != "build"
}
