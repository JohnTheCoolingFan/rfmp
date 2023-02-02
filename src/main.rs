use clap::Parser;
use glob::glob;
use mtzip::ZipArchive;
use serde::Deserialize;
use serde_json::from_reader;
use std::{
    error::Error,
    fs,
    io::BufWriter,
    path::{Path, PathBuf},
};
use walkdir::{DirEntry, WalkDir};

#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
struct CliArgs {
    #[clap(
        short,
        long,
        value_name = "PATH",
        help = "Install mod to <PATH> instead of default path",
        long_help = "Install mod to <PATH> instead of default path.\nDefault path is `$HOME/.factorio/mods` on linux and `{{FOLDERID_RoamingAppData}}\\Factorio\\mods`."
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

fn main() -> Result<(), Box<dyn Error>> {
    let cli_args = CliArgs::parse();

    // Mods directory path
    let mut zip_file_path = cli_args.install_dir.clone().unwrap_or_else(|| {
        if cfg!(target_os = "linux") {
            dirs::home_dir().unwrap().join(".factorio/mods")
        } else if cfg!(target_os = "windows") {
            dirs::data_dir().unwrap().join("Factorio/mods")
        } else {
            PathBuf::from(".")
        }
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
    let mod_name_with_version = format!("{mod_name}_{mod_version}");

    // Check for other versions
    if !cli_args.no_clean {
        // Check if any version of the mod already installed/exist.
        let mod_glob_str = format!(
            "{}/{}_*[0-9].*[0-9].*[0-9].zip",
            zip_file_path.to_string_lossy(),
            mod_name
        );
        let mod_glob = glob(&mod_glob_str)?;

        // Delete if any other versions found
        for entry in mod_glob {
            let entry = entry?;
            let entry_name = entry.to_string_lossy();
            println!("Removing {entry_name}");
            if entry.is_file() {
                fs::remove_file(&entry)?;
            } else {
                println!("Failed to remove {entry_name}: not a file");
            }
        }
    }

    // Mod file name
    let zip_file_name = format!("{mod_name_with_version}.zip");
    zip_file_path.push(&zip_file_name);

    // Walkdir iter, filtered
    let walkdir = WalkDir::new(".");
    let it = walkdir
        .into_iter()
        .filter_entry(|e| !is_hidden(e, &zip_file_name, &cli_args.exclude))
        .map(Result::unwrap)
        .map(|de| de.path().to_path_buf())
        .skip(1);

    // As testing found out, removing the file beforehand speeds up the whole process
    // Delete existing file. This probably wouldn't run unless --no-clean argument is passed.
    if zip_file_path.exists() {
        println!("{} exists, removing.", zip_file_path.to_string_lossy());
        if zip_file_path.is_file() {
            fs::remove_file(&zip_file_path)?;
        } else if zip_file_path.is_dir() {
            // Is this even possible?
            fs::remove_dir(&zip_file_path)?;
        }
    }

    // Create archive
    let zipwriter = ZipArchive::default();

    // Add root dir
    //println!("Adding root dir");
    zipwriter.add_directory(&mod_name_with_version);

    let path_prefix = Path::new(&mod_name_with_version);

    // Let the zipping begin!
    for path in it {
        let zip_path = path_prefix.join(path.strip_prefix("./")?);
        let zipped_name = zip_path.to_string_lossy();

        if path.is_file() {
            //println!("adding file {:?}", zipped_name);
            zipwriter.add_file(path, &zipped_name);
        } else if !path.as_os_str().is_empty() {
            //println!("adding dir  {:?}", zipped_name);
            zipwriter.add_directory(&zipped_name);
        }
    }

    // Create mod file
    let mut zip_file = BufWriter::new(fs::File::create(zip_file_path)?);

    // Finish writing
    zipwriter.write(&mut zip_file);

    Ok(())
}

// Function to filter all files we don't want to add to archive
fn is_hidden(entry: &DirEntry, zip_file_name: &str, excludes: &[PathBuf]) -> bool {
    let entry_file_name = entry.file_name().to_str().unwrap();
    entry_file_name == zip_file_name
        || (entry_file_name != "." && entry_file_name.starts_with('.'))
        || excludes.contains(&entry.path().into())
}
