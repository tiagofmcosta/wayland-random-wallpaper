use std::env;
use std::fs::{self, File};
use std::io::{BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};

use rand_core::OsRng;
use rand_distr::Distribution;
use rand_distr::Uniform;

const APP_NAME: &str = "Random Background";

const COMMAND_BACKGROUND_CHANGER: &str = "swww";
const COMMAND_NOTIFY_SEND: &str = "notify-send";

const TRANSITION_TYPE: &str = "any";
const TRANSITION_STEP: &str = "30";
const TRANSITION_DURATION: &str = "3";
const TRANSITION_FPS: &str = "165";

const EXPIRE_TIME: &str = "3000";

const ENV_VAR_NAME_CACHE_FILE_PATH: &str = "RB_CACHE_FILE_PATH";
const ENV_VAR_NAME_WALLPAPER_DIRECTORY_PATH: &str = "RB_WALLPAPER_DIRECTORY_PATH";

fn get_cache_file_path() -> PathBuf {
    let env_cache_file_path = env::var(ENV_VAR_NAME_CACHE_FILE_PATH);
    let mut path = "~/.background".to_string();

    if let Ok(env_path) = env_cache_file_path {
        path = env_path
    }

    PathBuf::from(shellexpand::tilde(&path).to_string())
}

fn get_previously_selected_background(cache_file_path: &PathBuf) -> String {
    let mut previous_background = String::new();
    if let Ok(mut file) = File::open(cache_file_path) {
        BufReader::new(&mut file)
            .read_to_string(&mut previous_background)
            .expect("Failed to read cache file.");
    }
    previous_background
}

fn get_wallpaper_directory_path() -> PathBuf {
    let env_wallpaper_directory_path = env::var(ENV_VAR_NAME_WALLPAPER_DIRECTORY_PATH);
    let mut path = "~/Pictures/wallpapers".to_string();

    if let Ok(env_path) = env_wallpaper_directory_path {
        path = env_path;
    }

    PathBuf::from(shellexpand::tilde(&path).to_string())
}

fn get_possible_backgrounds(
    previous_background: String,
    wallpaper_directory_path: &PathBuf,
) -> Vec<PathBuf> {
    fs::read_dir(wallpaper_directory_path)
        .unwrap_or_else(|_| panic!("Failed to open {}", &wallpaper_directory_path.display()))
        .filter_map(|entry| {
            if let Ok(dir_entry) = entry {
                let path = dir_entry.path();
                if path.is_file() {
                    Some(path)
                } else {
                    None
                }
            } else {
                None
            }
        })
        .filter(|file_path| is_image(file_path))
        .filter(|file_path| {
            file_path
                != wallpaper_directory_path
                    .join(&previous_background)
                    .as_path()
        })
        .collect::<Vec<_>>()
}

fn is_image(path: &Path) -> bool {
    match path.extension() {
        Some(ext) => {
            matches!(
                ext.to_str(),
                Some("jpg") | Some("jpeg") | Some("png") | Some("gif") | Some("bmp")
            )
        }
        _ => false,
    }
}

fn choose_random_background(possible_backgrounds: &Vec<PathBuf>) -> &PathBuf {
    let distribution = Uniform::new(0, possible_backgrounds.len());
    &possible_backgrounds[distribution.sample(&mut OsRng)]
}

fn get_file_name(selected_file: &&PathBuf) -> String {
    selected_file
        .file_name()
        .unwrap_or_else(|| panic!("Failed to get file name from {}.", selected_file.display()))
        .to_string_lossy()
        .to_string()
}

fn apply_new_background(cache_file_path: &PathBuf, selected_file: &&PathBuf, file_name: String) {
    let status = execute_background_changer(selected_file);

    if status.success() {
        update_cache(cache_file_path, &file_name);
        send_notification(&selected_file, file_name);
    } else {
        println!("{} execution failed.", COMMAND_BACKGROUND_CHANGER);
    }
}

fn execute_background_changer(selected_file: &&PathBuf) -> ExitStatus {
    Command::new(COMMAND_BACKGROUND_CHANGER)
        .arg("img")
        .args(["--transition-type", TRANSITION_TYPE])
        .args(["--transition-step", TRANSITION_STEP])
        .args(["--transition-duration", TRANSITION_DURATION])
        .args(["--transition-fps", TRANSITION_FPS])
        .arg(selected_file)
        .status()
        .unwrap_or_else(|_| panic!("Failed to execute {}.", COMMAND_BACKGROUND_CHANGER))
}

fn update_cache(cache_file_path: &PathBuf, file_name: &String) {
    let mut cache_file = File::create(cache_file_path)
        .unwrap_or_else(|_| panic!("Failed to create cache file {}.", cache_file_path.display()));

    cache_file
        .write_all(file_name.as_bytes())
        .unwrap_or_else(|_| panic!("Failed to update cache in {}", cache_file_path.display()));
}

fn send_notification(selected_file: &&&PathBuf, file_name: String) {
    Command::new(COMMAND_NOTIFY_SEND)
        .args(["-i", &selected_file.to_string_lossy()])
        .args(["-t", EXPIRE_TIME])
        .arg(APP_NAME)
        .arg(file_name)
        .status()
        .unwrap_or_else(|_| panic!("Failed to execute {COMMAND_NOTIFY_SEND}."));
}

fn main() {
    let cache_file_path = get_cache_file_path();
    let previous_background = get_previously_selected_background(&cache_file_path);
    let wallpaper_directory_path = get_wallpaper_directory_path();
    let possible_backgrounds =
        get_possible_backgrounds(previous_background, &wallpaper_directory_path);

    if possible_backgrounds.is_empty() {
        println!(
            "No images found in {}.",
            &wallpaper_directory_path.display()
        );
        return;
    }

    let selected_file = choose_random_background(&possible_backgrounds);
    let file_name = get_file_name(&selected_file);

    apply_new_background(&cache_file_path, &selected_file, file_name);
}
