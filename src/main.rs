use std::{
    ffi::OsStr,
    fmt::Display,
    fs::{self, File},
    io::BufWriter,
    num::NonZeroUsize,
    path::{Path, PathBuf},
};

use clap::{builder::TypedValueParser, Parser};
use glob::glob;
use mtzip::{level::CompressionLevel, CompressionType, ZipArchive};
use rayon::ThreadPoolBuilder;
use serde::Deserialize;
use serde_json::from_reader;
use walkdir::{DirEntry, WalkDir};

#[derive(Debug, Clone, Parser)]
#[clap(author, version, about, long_about = None)]
struct CliArgs {
    /// Install mod to <PATH> instead of default path.
    ///
    /// Default path is `$HOME/.factorio/mods` on linux and
    /// `{{FOLDERID_RoamingAppData}}\Factorio\mods`. Takes priority over $FACTORIO_MODS_HOME
    /// environment variable
    #[clap(short, long, value_name = "PATH", env = "FACTORIO_MODS_HOME")]
    install_dir: Option<PathBuf>,

    /// Do not search for other versions of the mod and do not try to remove them.
    #[clap(short, long, alias = "no-clean")]
    keep_old_versions: bool,

    /// Exclude files or directories from being included in teh archive
    #[clap(short, long, value_name = "PATH")]
    exclude: Vec<PathBuf>,

    // SAFETY: value range is restricted when clap parses an integer
    /// Set compression level to use instead of default.
    ///
    /// Default is best compression, 9.
    #[clap(short, long, value_parser = clap::value_parser!(u8).range(0..=9).map(|v| unsafe { CompressionLevel::new_unchecked(v) }))]
    level: Option<CompressionLevel>,

    /// Don't compress any data.
    ///
    /// Stored is a "compression" level where the file data is stored directly without any
    /// compression.
    #[clap(short, long)]
    stored: bool,

    /// Amount of threads that will be used for compression.
    #[clap(short, long)]
    threads: Option<NonZeroUsize>,
}

#[derive(Deserialize)]
struct InfoJson {
    name: String,
    version: String,
}

impl Display for InfoJson {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}_{}", self.name, self.version)
    }
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

/// Mods directory path
fn get_target_dir(install_dir: Option<PathBuf>) -> PathBuf {
    let mods_target_dir = install_dir.unwrap_or_else(get_default_factorio_home);

    if !mods_target_dir.exists() {
        panic!("Error: {} doesn't exist", mods_target_dir.to_string_lossy());
    }

    mods_target_dir
}

fn make_glob_str(target_dir: &Path, mod_name: &str) -> String {
    format!(
        "{}/{}_*[0-9].*[0-9].*[0-9].zip",
        target_dir.to_string_lossy(),
        mod_name
    )
}

fn remove_old_versions(target_dir: &Path, mod_name: &str) {
    let mod_glob_str = make_glob_str(target_dir, mod_name);
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

/// Walkdir iter, filtered
fn make_walkdir_iter<'a>(
    zip_file_name: &'a str,
    extra_exclude: &'a [PathBuf],
) -> impl Iterator<Item = PathBuf> + 'a {
    WalkDir::new(".")
        .into_iter()
        .filter_entry(|e| !walkdir_filter(e, zip_file_name, extra_exclude))
        .filter_map(|de_res| match de_res {
            Ok(de) => Some(de.path().to_path_buf()),
            Err(e) => {
                eprintln!("Error when walking the directory: {e}");
                None
            }
        })
        .skip(1)
}

fn set_new_thread_pool(threads: usize) {
    ThreadPoolBuilder::new()
        .num_threads(threads)
        .build_global()
        .unwrap()
}

fn main() {
    let cli_args = CliArgs::parse();
    #[cfg(debug_assertions)]
    println!("{cli_args:?}");
    let CliArgs {
        install_dir,
        keep_old_versions,
        exclude,
        level,
        stored,
        threads,
    } = cli_args;

    if let Some(threads) = threads.map(NonZeroUsize::get) {
        set_new_thread_pool(threads);
    }

    let mods_target_dir = get_target_dir(install_dir);

    let info_json = get_info_json();

    // Get mod name/id and version
    let mod_name_with_version = info_json.to_string();

    // Check for other versions
    if !keep_old_versions {
        remove_old_versions(&mods_target_dir, &info_json.name)
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
    let mut zipwriter = ZipArchive::default();

    // Add root dir
    //println!("Adding root dir");
    zipwriter.add_directory(mod_name_with_version.clone(), None);

    let path_prefix = Path::new(&mod_name_with_version);

    let walkdir = make_walkdir_iter(&zip_file_name, &exclude);

    // Let the zipping begin!
    for path in walkdir {
        let zip_path = path_prefix.join(
            path.strip_prefix("./")
                .expect("Failed to strip './' prefix"),
        );
        let zipped_name = zip_path.to_string_lossy();

        if path.is_file() {
            //println!("adding file {:?}", zipped_name);
            zipwriter.add_file_from_fs(
                path,
                zipped_name.to_string(),
                level,
                stored.then_some(CompressionType::Stored),
            );
        } else if !path.as_os_str().is_empty() {
            //println!("adding dir  {:?}", zipped_name);
            zipwriter
                .add_directory_with_metadata_from_fs(zipped_name.to_string(), path)
                .unwrap();
        }
    }

    // Create mod file
    let mut zip_file =
        BufWriter::new(File::create(target_zip_file).expect("Failed to open output file"));

    zipwriter.write_with_rayon(&mut zip_file).unwrap();
}

/// Function to filter all files we don't want to add to archive
fn walkdir_filter(entry: &DirEntry, zip_file_name: &str, excludes: &[PathBuf]) -> bool {
    let entry_path = entry.path();
    let filename = entry.file_name();
    is_filename_eq(filename, zip_file_name)
        || is_hidden(entry_path, filename)
        || is_in_excludes(entry_path, excludes)
}

fn is_filename_eq(filename: &OsStr, rhs: &str) -> bool {
    filename.to_str().map(|v| v == rhs).unwrap_or(false)
}

fn is_hidden(path: &Path, filename: &OsStr) -> bool {
    path != AsRef::<Path>::as_ref(&".")
        && filename
            .to_str()
            .map(|filename| filename.starts_with('.'))
            .unwrap_or(false)
}

fn is_in_excludes(path: &Path, excludes: &[PathBuf]) -> bool {
    excludes.iter().any(|e| path.starts_with(e))
}
