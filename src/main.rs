use std::{fs, path::{Path, PathBuf}, time::Instant, error::Error};
use mtzip::ZipArchive;
use serde_json::from_reader;
use serde::Deserialize;
use walkdir::{DirEntry, WalkDir};
use glob::glob;
use sysinfo::SystemExt;
use clap::Parser;

#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
struct CliArgs {
    #[clap(short, long, value_parser, value_name = "PATH", help = "Install mod to <PATH> instead of default path", long_help = "Install mod to <PATH> instead of default path.\nDefault path is `$HOME/.factorio/mods` on linux and `{{FOLDERID_RoamingAppData}}\\Factorio\\mods`.")]
    install_dir: Option<String>,

    #[clap(short, long, action, help = "Do not search for other versions of the mod and do not try to remove them.")]
    no_clean: bool,

    #[clap(short, long, action, help = "Measure how long compression takes.")]
    measure_time: bool
}

#[derive(Deserialize)]
struct InfoJson {
    name: String,
    version: String
}

fn main() -> Result<(), Box<dyn Error>>{
    let cli_args = CliArgs::parse();

    // Mods directory path
    let mut zip_file_path = cli_args.install_dir.map(PathBuf::from).unwrap_or_else(||
    if cfg!(target_os="linux") {
        dirs::home_dir().unwrap().join(".factorio/mods")
    }
    else if cfg!(target_os="windows") {
        dirs::data_dir().unwrap().join("Factorio/mods")
    }
    else {
        PathBuf::from(".")
    });

    if !zip_file_path.exists() {
        panic!("Error: {:?} doesn't exist", zip_file_path);
    }

    // Open info.json and parse it
    let info_file = fs::File::open("info.json")?;
    let info_json: InfoJson = from_reader(info_file)?;

    // Get mod name/id and version
    let mod_name = &info_json.name;
    let mod_version = &info_json.version;
    let mod_name_with_version = format!("{}_{}", mod_name, mod_version);
    
    // Check for other versions
    if !cli_args.no_clean {
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
    let zip_file_name = format!("{}.zip", mod_name_with_version);
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
    zipwriter.add_directory(&mod_name_with_version);

    let time_zip_measure = Instant::now();

    // Let the zipping begin!
    for entry in it {
        let entry = entry.unwrap();
        let name = entry.path();
        let zip_path = Path::new(&mod_name_with_version).join(&name.to_str().unwrap()[2..]);
        let zipped_name = zip_path.to_str().unwrap();

        if name.is_file() {
            //println!("adding file {:?}", zipped_name);
            zipwriter.add_file(name, zipped_name);
        } else if !name.as_os_str().is_empty() {
            //println!("adding dir  {:?}", zipped_name);
            zipwriter.add_directory(zipped_name);
        }
    }

    let threads = {
        let ref_kind = sysinfo::RefreshKind::new().with_cpu(sysinfo::CpuRefreshKind::new());
        let sys = sysinfo::System::new_with_specifics(ref_kind);
        sys.cpus().len()
    };

    // Finish writing
    zipwriter.write(&mut zip_file, Some(threads));

    if cli_args.measure_time {
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
