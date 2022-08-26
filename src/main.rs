use std::{fs, env, path::{Path, PathBuf}, time::Instant, error::Error};
use mtzip::ZipArchive;
use serde_json::from_reader;
use serde::Deserialize;
use walkdir::{DirEntry, WalkDir};
use glob::glob;

fn print_help(executable_name: &str, exit_code: i32) {
    println!("Usage: {} [--install-dir PATH] [--no-clean]\n\n    \
                            No arguments: Pack mod from mod files in current path (pwd) and install into default mod path.\n\n    \
                            --install-dir PATH: Install mod to PATH instead of default one.\n      \
                            Default path is `$HOME/.factorio/mods` and `{{FOLDERID_RoamingAppData}}\\Factorio\\mods`\n\n    \
                            --no-clean: Do not search for other versions of the mod and do not remove them.\n\n    \
                            --help: Show this message.\n\n    \
                            --measure-time: measure duration of compression.", executable_name);
    std::process::exit(exit_code);
}

#[derive(Deserialize)]
struct InfoJson {
    name: String,
    version: String
}

fn main() -> Result<(), Box<dyn Error>>{
    // Flags for args
    let mut check_old_versions = true;
    let mut next_path = false;
    let mut measure_time = false;

    // Mods directory path
    let mut zip_file_path = if cfg!(target_os="linux") {
        dirs::home_dir().unwrap().join(PathBuf::from(".factorio/mods"))
    }
    else if cfg!(target_os="windows") {
        dirs::data_dir().unwrap().join(PathBuf::from("Factorio/mods"))
    }
    else {
        PathBuf::from(".")
    };

    // Collect args
    let mut args = env::args();

    // Parse args
    if args.len() > 1 {
        // This requires more reliability, especially user input checking.
        let executable_name = args.next().unwrap();
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
                    "--measure-time" => measure_time = true,
                    _ => print_help(&executable_name, 1),
                }
            }
        }
    }

    if !zip_file_path.exists() {
        panic!("Error: {:?} doesn't exist", zip_file_path);
    }

    // Open info.json and parse it
    let info_file = fs::File::open("info.json")?;
    let info_json: InfoJson = from_reader(info_file)?;

    // Get mod name/id and version
    let mod_name = info_json.name;
    let mod_version = info_json.version;
    
    // Check for other versions
    if check_old_versions {
        // Check if any version of the mod already installed/exist.
        let mod_glob_str = format!("{}/{}_*[0-9].*[0-9].*[0-9].zip", zip_file_path.to_str().unwrap(), mod_name);
        let mod_glob = glob(&mod_glob_str)?;

        // Delete if any other versions found
        for entry in mod_glob {
            let entry = entry?;
            let entry_name = entry.to_str().unwrap();
            println!("Removing {}", entry_name);
            if entry.is_file() {
                fs::remove_file(&entry)?;
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
    let mut it = walkdir.into_iter().filter_entry(|e| !is_hidden(e, &zip_file_name));
    it.next();

    // As testing found out, removing the file beforehand speeds up the whole process
    // Delete existing file. This probably wouldn't run unless --no-clean argument is passed.
    if zip_file_path.exists() {
        println!("{} exists, removing.", zip_file_path.to_str().unwrap());
        if zip_file_path.is_file() {
            fs::remove_file(&zip_file_path)?;
        } else if zip_file_path.is_dir() { // Is this even possible?
            fs::remove_dir(&zip_file_path)?;
        }
    }

    // Create mod file
    let mut zip_file = fs::File::create(zip_file_path)?;

    // Create archive
    let zipwriter = ZipArchive::default();

    // Add root dir
    //println!("Adding root dir");
    zipwriter.add_directory(&format!("{}_{}", mod_name, mod_version));

    let time_zip_measure = Instant::now();

    // Let the zipping begin!
    for entry in it {
        let entry = entry.unwrap();
        let name = entry.path();
        let zip_path = Path::new(&format!("{}_{}", mod_name, mod_version)).join(&name.to_str().unwrap()[2..]);
        let zipped_name = zip_path.to_str().unwrap();

        if name.is_file() {
            //println!("adding file {:?}", zipped_name);
            zipwriter.add_file(name, zipped_name);
        } else if !name.as_os_str().is_empty() {
            //println!("adding dir  {:?}", zipped_name);
            zipwriter.add_directory(zipped_name);
        }
    }

    // Finish writing
    //zipwriter.finish()?;
    zipwriter.compress(12);
    zipwriter.write(&mut zip_file);

    if measure_time {
        println!("{}", time_zip_measure.elapsed().as_secs_f64());
    }

    Ok(())
}

// Function to filter all files we don't want to add to archive
fn is_hidden(entry: &DirEntry, zip_file_name: &str) -> bool {
    let entry_file_name = entry.file_name().to_str().unwrap();
    entry_file_name == zip_file_name ||
        (entry_file_name != "." && entry_file_name.starts_with('.')) ||
        entry_file_name == "build"
}
