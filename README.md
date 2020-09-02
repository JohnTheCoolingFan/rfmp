# Rust factorio Mod Packer

This program packs all files in current direcory (excluding unix-style hidden directories to ignore git stuff) into a properly-formatted mod and puts it into mods folder (`$HOME/.factorio/mods` on linux).
Currently (0.1.0) works linux-only.

### Mod File Structure Example
`<mod_name>` and `<mod_version>` are `"name"` and `"version"` values from mod's `info.json` file.
- `<mod_name>_<mod_version>.zip`
    - `<mod_name>_<mod_version>`
        - `info.json`
        - `data.lua`
        - `control.lua`
        - etc

### TODO:
- Command-line args
- Windows compatibility
- *MacOs?*