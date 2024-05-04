# Rust Factorio Mod Packer

Idea behind this project was to create a compiled and fast replacement to my shell script that packed factorio mods (very useful in my Factorio mod development pipeline).
This program packs all files in current direcory (excluding unix-style hidden directories to ignore git stuff) into a properly-formatted mod and puts it into mods folder (`$HOME/.factorio/mods` on Linux or `%AppData%\Factorio\mods` on Windows).
Works on Linux and Windows, builds are included on Releases page. MacOS support isn't planned.

All versions are available on GitHub releases page.

### Mod File Structure Example
`<mod_name>` and `<mod_version>` are `"name"` and `"version"` values from mod's `info.json` file.
- `<mod_name>_<mod_version>.zip`
    - `<mod_name>_<mod_version>`
        - `info.json`
        - `data.lua`
        - `control.lua`
        - etc

### Speed
This program manages to outperform (i.e. do the job faster) [my 7zip-based build script](https://gist.github.com/JohnTheCoolingFan/eb0587b1156b137cb1cbf7111e82d14b) on my machine (Ryzen 5 3600). This is achieved by using library [mtzip](https://crates.io/crates/mtzip) (also made by me). It splits the file compression jobs into tasks that can be run concurrently.

## Rayon support

[`rayon`](https://crates.io/crates/rayon) is supported as an alternative  to `mtzip`'s thread management. It can be enabled with `rayon` feature, replaces the default behavior. From my personal testing, it results in a lightly lower performance in extreme situations, but nothing that can be noticed without timing the execution time.
