# Credits

## Console photos

Photographs from Wikimedia Commons, resized and trimmed (the GameCube photo also had its white background removed).

| System | File | Author | License | Source |
|---|---|---|---|---|
| Nintendo 64 | `static/consoles/n64.png` | Evan-Amos | Public domain | https://commons.wikimedia.org/wiki/File:N64-Console-Set.png |
| GameCube | `static/consoles/gc.png` | Evan-Amos | Public domain | https://commons.wikimedia.org/wiki/File:GameCube-Set.jpg |
| Super Nintendo | `static/consoles/snes.png` | Evan-Amos | CC BY-SA 3.0 | https://commons.wikimedia.org/wiki/File:SNES-Mod1-Console-Set.png |
| Game Boy Advance | `static/consoles/gba.png` | Evan-Amos | Public domain | https://commons.wikimedia.org/wiki/File:Nintendo-Game-Boy-Advance-Purple-FL.png |
| PlayStation | `static/consoles/ps1.png` | Evan-Amos | Public domain | https://commons.wikimedia.org/wiki/File:PSX-Console-wController.png |
| PlayStation 2 | `static/consoles/ps2.png` | Evan-Amos | Public domain | https://commons.wikimedia.org/wiki/File:PS2-Fat-Console-Set.png |
| Mega Drive | `static/consoles/md.png` | Evan-Amos | Public domain | https://commons.wikimedia.org/wiki/File:Sega-Mega-Drive-EU-Mk1-wController-FL.png |
| Xbox 360 | `static/consoles/x360.png` | Evan-Amos, modified by Gunnar.offel | Public domain | https://commons.wikimedia.org/wiki/File:Microsoft-Xbox-360-Pro-Console-FL.png |
| NES | `static/consoles/nes.png` | Evan-Amos | Public domain | https://commons.wikimedia.org/wiki/File:NES-Console-Set.png |
| Game Boy | `static/consoles/gb.png` | Evan-Amos | Public domain | https://commons.wikimedia.org/wiki/File:Game-Boy-Original.png |
| Wii | `static/consoles/wii.png` | Evan-Amos | Public domain | https://commons.wikimedia.org/wiki/File:Wii-Console.png |
| Arcade | `static/consoles/arcade.png` | Fan de Rétro | CC BY 4.0 | https://commons.wikimedia.org/wiki/File:Borne_arcade_Pacman.png |

CC BY-SA 3.0 images are shared under the same license: https://creativecommons.org/licenses/by-sa/3.0/. CC BY 4.0: https://creativecommons.org/licenses/by/4.0/

## Cartridge and disc templates

`static/media/*-cartridge.png` and `static/media/*-disc.png` are label templates found on image search (the N64 shell is a photograph); their label windows and disc holes are transparent so the cover art can sit behind them. Sources: pngfind.com (SNES), pngkey.com (PS2), and a disc template by stanrebro on DeviantArt (Xbox 360); the others came without attribution. The NES shell is Evan-Amos's public domain photograph [NES-Cartridge.jpg](https://commons.wikimedia.org/wiki/File:NES-Cartridge.jpg) with its label recess opened; the Game Boy cartridge, the Wii disc and the arcade marquee frame were drawn for PortShelf (`static/media/gb-cartridge.png`, `wii-disc.png`, `arcade-cartridge.png`).

## System logos

`static/logos/*.png` are single-colour masks made from logos on Wikimedia Commons, all public domain (text logos): [Nintendo 64 wordmark](https://commons.wikimedia.org/wiki/File:Nintendo_64_wordmark.svg), [Nintendo GameCube Official Logo](https://commons.wikimedia.org/wiki/File:Nintendo_GameCube_Official_Logo.svg), [Super Nintendo Entertainment System logo](https://commons.wikimedia.org/wiki/File:Super_Nintendo_Entertainment_System_logo.svg), [Game Boy Advance logo](https://commons.wikimedia.org/wiki/File:Game_Boy_Advance_logo.svg), [PlayStation logo and wordmark](https://commons.wikimedia.org/wiki/File:PlayStation_logo_and_wordmark.svg), [PlayStation 2 logo](https://commons.wikimedia.org/wiki/File:PlayStation_2_logo.svg), [Mega Drive (Japan) logo](https://commons.wikimedia.org/wiki/File:MegaDriveJPLogo.svg) [Xbox 360 wordmark](https://commons.wikimedia.org/wiki/File:Xbox_360_wordmark.svg), [NES logo](https://commons.wikimedia.org/wiki/File:NES_logo.svg), [Nintendo Game Boy Logo](https://commons.wikimedia.org/wiki/File:Nintendo_Game_Boy_Logo.svg) and [Wii](https://commons.wikimedia.org/wiki/File:Wii.svg). Arcade has no system logo; its wordmark is set in [Press Start 2P](https://fonts.google.com/specimen/Press+Start+2P) by CodeMan38 (SIL Open Font License 1.1). The trademarks belong to their owners.

## Fonts

Both under the SIL Open Font License 1.1, Latin subsets from Google Fonts:

- [Outfit](https://fonts.google.com/specimen/Outfit) by Rodrigo Fuenzalida, for headings (`static/fonts/outfit.woff2`)
- [Atkinson Hyperlegible Next](https://fonts.google.com/specimen/Atkinson+Hyperlegible+Next) by the Braille Institute, for text (`static/fonts/atkinson-next.woff2`)

The achievement card drawn over games uses fixed-weight TTF copies of the same two fonts, from the [Google Fonts repository](https://github.com/google/fonts) (`src-tauri/assets/fonts/`).

## Box art

Box art is not bundled. The shelf reads covers from `~/.config/portshelf/covers/`; the ones used during development come from [libretro-thumbnails](https://github.com/libretro-thumbnails).
