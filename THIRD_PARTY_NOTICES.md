# Licensing scope and third-party notices

## Publication status

**Unresolved: this repository has not been cleared for public redistribution
as a complete work.** The project license covers only original contributions
that their copyright holders are entitled to license. It cannot supply missing
rights to translated code, third-party expression, game data or trademarks.
Neither a fresh Git history, attribution, a language change nor a requirement
to own the original game resolves those rights by itself.

Before distributing the complete source or a binary, establish the provenance
and redistribution basis of reference-derived portions, obtain any necessary
permissions, and satisfy the applicable dependency licenses. Remove or replace
material lacking a redistribution basis only after identifying its scope;
retain attribution and evidence of origin. This is not a legal opinion or a
statement that any particular portion has been adjudicated infringing.

## GTA III, re3 and reVC references

Grand Theft Auto, Grand Theft Auto III and related names and content belong to
their respective rights holders. No rights to Rockstar Games or Take-Two
Interactive game data, source code, trademarks, music, dialogue or other
content are granted by the project license. A local game installation is an
input to the software, not part of its licensed source distribution.

Development has used these reverse-engineering projects as source references:

- [Jai-JAP/re-GTA](https://github.com/Jai-JAP/re-GTA), local reference revision
  `d0e4e15c815afe1033b3322aac492e28c72ab5b8`. Its README license section says
  its authors do not feel in a position to give the code a license. Do not
  treat this as a standard open-source redistribution grant.
- [mrxenginner/reVC](https://github.com/mrxenginner/reVC), local reference
  revision `a26fff4af555a8a425ac9f17d1dd718d00d1f942`. Its local `LICENSE.md`
  contains a custom "MIT License with Proof-of-Usage Condition (MIT-PoU)";
  this is not the unmodified MIT license. Its conditions and its authors'
  authority over underlying material require separate evaluation.

Reference trees are optional local development inputs. They are not linked
into the game binary and must not be copied into a source or binary release.
The source exporter also omits the optional reference-derived C++ parity
harness while preserving the local development copy. Excluding those inputs
does not establish redistribution rights for material
already translated into Rust, shaders or the C++ parity harness.

Source reference comments record development provenance; they are not a claim
of clean-room implementation. Reference-derived identifiers, tables and
implementations, including `crates/re3bevy_game/src/core/MenuScreens.rs`,
`MenuScreensCustom.rs`, `Pools.rs`, `audio/AudioSamples.rs` and
`parity/cpp_harness/src/main.cpp`, need a provenance assessment. These examples
are not a complete list or a finding that every referenced item is protected.

## Vendored PL_MPEG

- Upstream: [phoboslab/pl_mpeg](https://github.com/phoboslab/pl_mpeg).
- Pinned source revision: `c871f2be022ece7ef4f64230b4fb8e1fb9eb6023`.
- Vendored source: `crates/re3bevy_formats/native/pl_mpeg.h`.
- Copyright (c) Dominic Szablewski (phoboslab.org).
- License: MIT, reproduced in
  [LICENSE.pl_mpeg](crates/re3bevy_formats/native/LICENSE.pl_mpeg).
- The project wrapper/adaptation is in `native/mpeg.c`; the upstream notice
  must accompany distributions containing the decoder.

## Cargo, Android and native runtime dependencies

Dependencies retain their own licenses. Cargo manifests and `Cargo.lock`
identify Rust dependencies; Android Gradle files identify Java/Android
dependencies. The project MIT license does not replace their terms.

Binary redistribution needs the license texts, notices and any required source
availability information for the actual build, including transitive, vendored
and native dependencies. In particular, Symphonia packages use MPL-2.0; inspect
their source and executable distribution requirements. Rust toolchain DLLs,
Microsoft runtimes and Android/Gradle dependencies also need assessment when
included. The notices copied by local build scripts are not a complete binary
license-compliance bundle. Do not publish `dist/` as a cleared release on that
basis.
