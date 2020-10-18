# Rust factorio Mod Packer

Idea behind this project was to create a compiled and fast replacement to my shell script that packed factorio mods (very useful in my Factorio mod development pipeline).
This program packs all files in current direcory (excluding unix-style hidden directories to ignore git stuff) into a properly-formatted mod and puts it into mods folder (`$HOME/.factorio/mods` on linux).
Current builds (0.1.3) are linux-only, but I managed to build it on windows.

All versions are available on GitLab releases page or [from my friend's server](https://cavej376.ddns.net/files/rfmp_releases/).

### Mod File Structure Example
`<mod_name>` and `<mod_version>` are `"name"` and `"version"` values from mod's `info.json` file.
- `<mod_name>_<mod_version>.zip`
    - `<mod_name>_<mod_version>`
        - `info.json`
        - `data.lua`
        - `control.lua`
        - etc

### TODO:
- Windows compatibility
- *MacOs?*
