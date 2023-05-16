# Sylv API
The backend for [sylv.gay](https://sylv.gay/).

## Mod ID Database

Searches Modrinth and other places for mod IDs and adds them to the mod ID database. This can be used to check if a mod ID has already been taken by another mod.

### Supported Mod Platforms
- [x] Modrinth
- [ ] CurseForge
    - Since CurseForge's API is closed, we cannot add support for it.
- [ ] <sub>~~Game Banana~~</sub>

### Supported Mod Loaders
- Fabric
- Quilt
- ~~Forge~~ (This may be added in the future, but not now)

### How do I upload my own mod ID?
Currently, you cannot. However, this a planned feature.

### MSRV <sub>why do you need an MSRV on a g~~ithub actio~~</sub>
We support Rust 1.69 because it is a pretty nice version.

## TODO

- [ ] Update this README with the details of this action
- [ ] Update inputs/outputs in `action.yaml`
- [ ] Implement the action's logic in `src/main.rs`
- [ ] Rename the default Git branch to `v1` (instead of `main` or `master`.) This helps with potential future breaking changes. **PROVIDED ACTIONS WILL NOT WORK UNTIL YOU DO THIS** 
