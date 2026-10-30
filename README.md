# AC6-FXR-DOOM-POC

ACVI but the TURNER and SCUDDER fire DOOM

Bottom text.

Doom (via [doomgeneric](https://github.com/ozkl/doomgeneric)) runs inside a DLL in Armored Core VI (1.09.1). Its
frames are shown on a 160x100 "screen" of 16,000 particles, each colored by custom FXR external values the DLL
provides. [AC6-FXR-DLL-POC](https://github.com/lugia19/AC6-FXR-DLL-POC) explains the technique with a far simpler
example; this repo pushes it as far as it goes.

## Try it

1. Download the prebuilt zip from the [Releases](../../releases) page and extract it anywhere.
2. Launch `fxr-doom.me3` with [me3](https://github.com/garyttierney/me3) (ModEngine 3). It loads `mod/` and
   `fxr_doom.dll`; Doom starts in the background with the game.
3. Equip **RF-024 TURNER** and **RF-025 SCUDDER**, one in each arm.
4. Stand still, fire TURNER (top half of the screen), then SCUDDER (bottom half) without moving. Their bullets hang
   in place for 24 hours. The screen fills in over a few seconds while you look at it.

The zip includes the shareware `doom1.wad`. If you own Doom or Doom II, put `doom.wad` or `doom2.wad` in `doom/`:
the DLL plays the first of `doom2.wad`, `doom.wad`, `doom1.wad` it finds.

### Controls

Only while the game window is focused. The keys also reach Armored Core, so they stay off its default keyboard
controls.

| Key | Doom |
|---|---|
| Arrows | Move / turn |
| Right Ctrl | Fire |
| Right Shift | Use / open doors |
| `,` `.` | Strafe left / right |
| 1-7 | Weapons |
| Enter | Menu: select |
| Backspace | Menu (Esc opens Armored Core's) |
| Y / N | Answer prompts |
| F8 | Turn Doom's input on/off |

Doom writes `default.cfg` and its saves to the game's working directory (`Game\`). Quitting Doom only stops Doom: the
screen keeps its last frame. The DLL logs to `Game\debug-logs\fxr_doom.log`.

## How it works

- **DLL** (`dll/`, Rust): doomgeneric, compiled in with the `cc` crate, runs on its own thread and copies each
  frame into a 320x200 array (`dll/src/doom.rs`). The DLL hooks the function effects read external values through
  (`0x142b56480`) and answers IDs `1000000 + (y * 320 + x) * 4 + channel` from that array, as 0..1.
- **Screen** (`fxr/build_screen.mjs`, [@cccode/fxr](https://github.com/EvenTorset/fxr)): one node per pixel, each a
  single camera-facing square particle whose color has three `ExternalValue1Modifier`s (R, G, B) reading its
  pixel's IDs. 160x100 samples every other pixel and row of Doom's 320x200.
- **Params** (`build.py`): TURNER's and SCUDDER's bullets (`Bullet` 10110000, 10120000) barely move and last a day,
  and spawn the screen's top (FXR 9990001) and bottom (9990002) halves.

## Gotchas

What it took to get here, for whoever tries something like it:

- **~11,500 particles per spawned effect.** Past that, the effect creates no more particles: a single 160x100 FXR
  stops after 72 rows. The limit covers child effects too: a root with two `ProxyNode`s pointing to two halves still
  stops at 72 rows, and both halves read through one value block. Each half needs a spawn of its own, hence two guns.
- **A global ceiling around 40k.** Four 11.5k screens crashed the game; three were fine.
- **Particles are created gradually, and only while the effect is in view.** A screen fired while looking away stays
  partial until you look at it; already-created particles stay.
- **Particles that live forever still update.** One particle per pixel from a `OneTimeEmitter` with duration -1,
  and its color modifiers are evaluated every frame. That's half the particles of re-emitting short-lived ones.
- **Frame rate:** 43 fps with one ~11.5k screen, ~36 with the full 160x100, 19 with several screens out (on the
  machine it was made on).
- **`add_sfxId_Bullet` doesn't spawn a second effect** on TURNER's bullet, and `numShoot` would spawn the same FXR
  twice. A second weapon was the simplest way to get a second, different effect.
- **The halves were lined up by measurement.** Each bullet starts at its own muzzle: with a temporary hook on the
  effect spawn function (`0x1416246f0(manager, handle_out, &fxr_id, transform, block, ...)`, transform = 4x4 matrix,
  rows: right, up, forward, position), SCUDDER's bullet starts 1.1 m further forward and 0.27 m higher than
  TURNER's. The bottom half is offset by that, in its bullet's frame. It only lines up when fired standing still.
- **Doom's `exit()` would close the game.** It's called on quit and on errors, so `dll/c/dg_platform.h` (forced
  into every Doom source file) redirects it to a function that ends only Doom's thread.

## Build

```
python build.py --game "<...>/ARMORED CORE VI FIRES OF RUBICON/Game" --witchy "<...>/WitchyBND.exe"
```

Needs Python 3, git, node/npm, cargo (stable, MSVC toolchain) and [WitchyBND](https://github.com/ividyon/WitchyBND)
3.x. The first run clones doomgeneric (pinned commit) into `dll/doomgeneric` and downloads the shareware `doom1.wad`
(SHA-1 checked) into `wad/`; put other WADs there too. It takes `regulation.bin` and
`sfx/sfxbnd_commoneffects.ffxbnd.dcx` from the game, adds the screen FXRs, patches the two bullets, repacks, builds
the DLL, and writes `package/` (what the release zip contains).

## License

The DLL contains doomgeneric, which is GPL-2.0 (its license is in the zip as `doom/doomgeneric-LICENSE.txt`), so
the DLL is GPL-2.0 too. `doom1.wad` is id Software's shareware episode.
