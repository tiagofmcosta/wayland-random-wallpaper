use std::env;
use std::fs::{self, File};
use std::io::{BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};

use heck::ToShoutySnakeCase;
use notify_rust::{Hint, Notification};
use rand_core::OsRng;
use rand_distr::Distribution;
use rand_distr::Uniform;
use tracing::{error, info, warn, Level};
use tracing_unwrap::{OptionExt, ResultExt};

use EnvVar::{CacheFile, WallpaperChanger, WallpaperFolder};

const APP_NAME: &str = "Random Wallpaper";

const TRANSITION_TYPE: &str = "any";
const TRANSITION_STEP: &str = "30";
const TRANSITION_DURATION: &str = "3";
const TRANSITION_FPS: &str = "165";

const EXPIRE_TIME: i32 = 3000;

#[derive(Debug)]
enum EnvVar {
    CacheFile,
    WallpaperFolder,
    WallpaperChanger,
}

impl ToString for EnvVar {
    #[tracing::instrument]
    fn to_string(&self) -> String {
        format!("RW_{:?}", self).to_shouty_snake_case()
    }
}

fn setup_tracing_subscriber() {
    let subscriber = tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber)
        .expect("Failed to set global tracing subscriber");
}

#[tracing::instrument]
fn get_value_from_env_var_or_default(env_var: EnvVar, default: &str) -> String {
    let env_value_result = env::var(env_var.to_string());
    if let Ok(env_value) = env_value_result {
        return env_value;
    }
    default.to_string()
}

#[tracing::instrument]
fn get_cache_file_path() -> PathBuf {
    let path = get_value_from_env_var_or_default(CacheFile, "~/.wallpaper");
    PathBuf::from(shellexpand::tilde(&path).to_string())
}

#[tracing::instrument]
fn get_previously_used_wallpaper(cache_file_path: &PathBuf) -> String {
    let mut previous_wallpaper = String::new();
    if let Ok(mut file) = File::open(cache_file_path) {
        BufReader::new(&mut file)
            .read_to_string(&mut previous_wallpaper)
            .expect_or_log("Failed to read cache file.");

        info!("Previously used wallpaper: {}", previous_wallpaper)
    }
    previous_wallpaper
}

#[tracing::instrument]
fn get_wallpaper_directory_path() -> PathBuf {
    let path = get_value_from_env_var_or_default(WallpaperFolder, "~/Pictures/wallpapers");
    PathBuf::from(shellexpand::tilde(&path).to_string())
}

#[tracing::instrument]
fn get_possible_wallpapers(
    previous_wallpaper: String,
    wallpaper_directory_path: &PathBuf,
) -> Vec<PathBuf> {
    fs::read_dir(wallpaper_directory_path)
        .expect_or_log(format!("Failed to open {}", &wallpaper_directory_path.display()).as_str())
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
        .filter(|file_path| file_path != Path::new(&previous_wallpaper))
        .collect::<Vec<_>>()
}

#[tracing::instrument]
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

#[tracing::instrument]
fn choose_random_wallpaper(possible_wallpapers: &Vec<PathBuf>) -> &PathBuf {
    let distribution = Uniform::new(0, possible_wallpapers.len());
    &possible_wallpapers[distribution.sample(&mut OsRng)]
}

#[tracing::instrument]
fn get_file_name(selected_file: &PathBuf) -> String {
    selected_file
        .file_name()
        .expect_or_log(
            format!("Failed to get file name from {}.", selected_file.display()).as_str(),
        )
        .to_string_lossy()
        .to_string()
}

#[tracing::instrument]
fn apply_new_wallpaper(cache_file_path: &PathBuf, selected_file: &PathBuf) {
    let command = get_value_from_env_var_or_default(WallpaperChanger, "swww");
    let status = execute_wallpaper_changer(&command, selected_file);

    if status.success() {
        update_cache(cache_file_path, selected_file);
        send_wallpaper_changed_notification(selected_file);
        info!(
            "Wallpaper successfully changed to {}",
            selected_file.display()
        )
    }
}

#[tracing::instrument]
fn execute_wallpaper_changer(command: &str, selected_file: &PathBuf) -> ExitStatus {
    Command::new(command)
        .arg("img")
        .args(["--transition-type", TRANSITION_TYPE])
        .args(["--transition-step", TRANSITION_STEP])
        .args(["--transition-duration", TRANSITION_DURATION])
        .args(["--transition-fps", TRANSITION_FPS])
        .arg(selected_file)
        .status()
        .expect_or_log(format!("Failed to execute {}.", command).as_str())
}

#[tracing::instrument]
fn update_cache(cache_file_path: &PathBuf, file_path: &PathBuf) {
    let mut cache_file = File::create(cache_file_path).expect_or_log(
        format!("Failed to create cache file {}.", cache_file_path.display()).as_str(),
    );

    cache_file
        .write_all(file_path.to_string_lossy().as_bytes())
        .expect_or_log(format!("Failed to update cache in {}", cache_file_path.display()).as_str());
}

#[tracing::instrument]
fn send_notification(body: &str, icon: &str, sticky: bool) {
    let mut notification_builder: &mut Notification = &mut Notification::new();
    notification_builder = notification_builder
        .summary(APP_NAME)
        .body(body)
        .icon(icon)
        .timeout(EXPIRE_TIME);

    if sticky {
        notification_builder = notification_builder
            .timeout(i32::MAX)
            .hint(Hint::Resident(true));
    }

    let result = notification_builder.finalize().show();
    if result.is_err() {
        error!("Failed to send notification.");
    }
}

#[tracing::instrument]
fn send_wallpaper_changed_notification(selected_file: &PathBuf) {
    send_notification(
        get_file_name(selected_file).as_str(),
        &selected_file.to_string_lossy(),
        false,
    )
}

fn main() {
    setup_tracing_subscriber();

    let cache_file_path = get_cache_file_path();
    let previous_wallpaper = get_previously_used_wallpaper(&cache_file_path);
    let wallpaper_directory_path = get_wallpaper_directory_path();
    let possible_wallpapers =
        get_possible_wallpapers(previous_wallpaper, &wallpaper_directory_path);

    if possible_wallpapers.is_empty() {
        warn!("No images found in {}", &wallpaper_directory_path.display());
        send_notification(
            format!("No images found in {}", &wallpaper_directory_path.display()).as_str(),
            "dialog-warning",
            true,
        );
        return;
    }

    let selected_file = choose_random_wallpaper(&possible_wallpapers);
    apply_new_wallpaper(&cache_file_path, selected_file);
}
