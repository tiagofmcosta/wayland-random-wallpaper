# Random Wallpaper

Simple application to randomly apply a wallpaper from a given folder.

Made for [Wayland](https://wayland.freedesktop.org/) on [Arch Linux](https://archlinux.org/)

Tested on [Hyprland](https://github.com/hyprwm/Hyprland) using [swww](https://github.com/Horus645/swww)

Supported extensions: `jpg`, `jpeg`, `png`, `gif`, `bmp`

## Configuration

| Environment Variable   | Description                                                                                  | Default                 |
|------------------------|----------------------------------------------------------------------------------------------|-------------------------|
| `RB_CACHE_FILE`        | Path for the cache file that prevents two consecutive runs from using the same wallpapers.   | `~/.wallpaper`          |
| `RB_WALLPAPER_FOLDER`  | Folder to look for wallpapers.                                                               | `~/Pictures/wallpapers` |
| `RB_WALLPAPER_CHANGER` | Path to the command to change the wallpaper with.                                            | `swww`                  |