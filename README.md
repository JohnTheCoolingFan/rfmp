# Rust Factorio Mod Packer

Idea behind this project was to create a compiled and fast replacement to my shell script that packed factorio mods (very useful in my Factorio mod development pipeline).
This program packs all files in current direcory (excluding unix-style hidden directories to ignore git stuff) into a properly-formatted mod and puts it into mods folder (`$HOME/.factorio/mods` on Linux or `%AppData%\Factorio\mods` on Windows).
Works on Linux and Windows, builds are included on Releases page. MacOS support isn't planned.

All versions are available on GitLab releases page or ~~from my friend's server~~. Releases have been moved to [Yandex.Disk](https://yadi.sk/d/smSSvKYreuQP4A) due to lack of hosting. If you have trouble downloading binaries, open an issue and I will find a new way to host binaries download.

### Mod File Structure Example
`<mod_name>` and `<mod_version>` are `"name"` and `"version"` values from mod's `info.json` file.
- `<mod_name>_<mod_version>.zip`
    - `<mod_name>_<mod_version>`
        - `info.json`
        - `data.lua`
        - `control.lua`
        - etc
