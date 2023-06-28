# Sylv API
The backend for [sylv.gay](https://sylv.gay/).

## Mod ID Database

Searches Modrinth and other places for mod IDs and adds them to the mod ID database. This can be used to check if a mod ID has already been taken by another mod.

### Supported Mod Platforms
- [x] Modrinth
- [ ] CurseForge
    - Since CurseForge's API is closed, we cannot add support for it.

### Supported Mod Loaders
- Fabric
- Quilt
- ~~Forge~~ (This may be added in the future, but not now)

### How do I upload my own mod ID?
Currently, you cannot. However, this a planned feature.

### MSRV
We support Rust 1.69 because it is a pretty nice version.

## TODO

- [x] Implement automatic mod searching.
- [ ] Allow the jar scan loop to scan by most popular mods with a saved, increasing offset over a short span of time.
- [ ] Include provided mod IDs.
- [ ] Search for dependencies.
- [ ] Check if queried mod IDs still exist.
