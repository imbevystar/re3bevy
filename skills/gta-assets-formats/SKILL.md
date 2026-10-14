---
name: gta-assets-formats
description: GTA original-install loading, format parsers, Bevy asset adaptation, streaming and deployment.
---

## Installation and deployment

- Game and tools share `OriginalGtaInstall`: `--gta-dir`, executable-adjacent configuration/installation, then working-directory configuration/installation. Support `config/original_gta3_path.txt`; never hardcode checkout paths.
- Executable/dist plus required runtime DLLs must work with an existing GTA III installation without repository `assets/` or `references/` trees. Project-owned shader code ships in the executable; original GTA files and reference C++ do not.
- Android APKs contain no original game data by default; the launcher imports a user-selected local installation into app-private storage. Asset-containing test APKs require `-BundleGameData`, use a distinct private filename and must not be published. Preserve project and upstream notices in local packages; those notices alone do not clear binary redistribution.
- Required missing/invalid data produces an error or named waiting/failure counter. Expected streaming waits remain distinguishable from malformed data; no silent empty/default/fabricated replacement.

## Parsing and runtime assets

- Verify the affected re3 loader's layout, endianness, signedness, alignment, sector units, limits, duplicate/order rules, strings and malformed-input behavior.
- Byte parsing belongs in `rustgta3_formats`; installation and Bevy adaptation belong in asset owners. Gameplay consumes resolved data, not duplicated parsers or installation-specific paths.
- Preserve TXD texture lookup/names, DFF model/material relations, residency and streaming through Bevy `Asset`, `Handle`, `Image` and resources. Extend that owner when incomplete; a directly loaded texture that bypasses GTA resolution is not a correct fix.
- Original files on disk may become native Bevy assets in memory. Do not add a mandatory offline conversion step. Future importer direction is in [re3-port-workflow](../re3-port-workflow/SKILL.md).

## File data versus executable behavior

Trace each affected input through the reference loading/draw paths. Read authored
textures, geometry/prelight, tables, animations and scripts from the installation;
port executable calculations into Rust/Bevy/WESL.

For example, `waterpro.dat` and the particle TXD's `water_old` are original data;
wind/time waves and UV scrolling are CWaterLevel calculations. TIMECYC supplies
sky colours; CClouds computes the background gradient. Neither fabricated data
nor deletion of executable calculations preserves this distinction.

For shader/material/draw changes, read [wesl-rendering](../wesl-rendering/SKILL.md).
Verify affected parsers with existing tests/real assets; loading or deployment
changes also need an installation-based run without checkout dependencies.
