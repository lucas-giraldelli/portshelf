# Banjo-Kazooie: game reference and achievement plan

Working notes for PortShelf's own Banjo-Kazooie set. The game content below comes from the Banjo-Kazooie decompilation (CC0) where marked, and from general knowledge of the game otherwise; entries marked "to confirm" are checked while playing.

## Worlds

| Order | World | Level id | Jiggies | Notes | Jinjos |
|---|---|---|---|---|---|
| hub | Spiral Mountain | 11 | 0 | 0 | 0 |
| hub | Gruntilda's Lair | 6 | 10 | 0 | 0 |
| 1 | Mumbo's Mountain | 1 | 10 | 100 | 5 |
| 2 | Treasure Trove Cove | 2 | 10 | 100 | 5 |
| 3 | Clanker's Cavern | 3 | 10 | 100 | 5 |
| 4 | Bubblegloop Swamp | 4 | 10 | 100 | 5 |
| 5 | Freezeezy Peak | 5 | 10 | 100 | 5 |
| 6 | Gobi's Valley | 7 | 10 | 100 | 5 |
| 7 | Mad Monster Mansion | 10 | 10 | 100 | 5 |
| 8 | Rusty Bucket Bay | 9 | 10 | 100 | 5 |
| 9 | Click Clock Wood | 8 | 10 | 100 | 5 |

Totals: 100 jiggies, 900 notes, 45 Jinjos. Level ids are the decomp's `level_e` values (current level at `0x80383301`).

Note doors in the lair open at 50, 180, 260, 350, 450, 640, 765 and 810 notes.

## Moves (decomp: `src/SM/ch/smbottles.c`, `src/core2/ch/mole.c`)

| Where Bottles teaches it | Moves |
|---|---|
| Spiral Mountain | Claw Swipe, Roll, Rat-a-tat Rap, Beak Barge, Feathery Flap, Flap Flip, higher jump (hold A), Climb, Dive, camera control |
| Mumbo's Mountain | Egg Firing, Beak Buster, Talon Trot |
| Treasure Trove Cove | Shock Spring Jump, Flight |
| Clanker's Cavern | Wonderwing |
| Bubblegloop Swamp | Wading Boots |
| Freezeezy Peak | Beak Bomb |
| Gobi's Valley | Turbo Talon Trot (Turbo Trainers) |

The lair's first note door also counts as a lesson in the code (`ABILITY_13_1ST_NOTEDOOR`).

## Mumbo's transformations (decomp: `transformation_e`)

| Transformation | Where |
|---|---|
| Termite | Mumbo's Mountain |
| Crocodile | Bubblegloop Swamp |
| Walrus | Freezeezy Peak |
| Pumpkin | Mad Monster Mansion |
| Bee | Click Clock Wood |
| Washing machine | exists in the code (`TRANSFORM_7_WISHWASHY`); where to get it: to confirm |

## Collectibles and extras

- Jiggies: 100 (one bit each at `0x803832C0`)
- Notes: 100 per world, 900 in total
- Jinjos: 5 per world
- Empty honeycombs: pieces that extend the life bar (24 in total, to confirm)
- Mumbo tokens: spent on transformations (116 in total, to confirm)
- Extra lives (1-up trophies) in every world
- Cheats spelled out in Treasure Trove Cove's sandcastle: blue eggs, red feathers, gold feathers, and Bottles' bonus cheats (big head, big hands and feet, big Kazooie, slim Banjo, and one more)
- Stop 'n' Swop: seven hidden items (six eggs and the ice key)
- Endings: the normal ending, the secret ending for all 100 jiggies, and the ending where the witch's quiz is lost

## Challenges and bosses (to confirm while playing)

- Conga (Mumbo's Mountain), Nipper (Treasure Trove Cove)
- Clanker's release (Clanker's Cavern)
- Mr. Vile's games and Tanktup (Bubblegloop Swamp)
- Boggy's sled race and Sir Slush (Freezeezy Peak)
- the Ancient Ones and Gobi (Gobi's Valley)
- Motzand's organ (Mad Monster Mansion)
- Boss Boom Box (Rusty Bucket Bay)
- Grunty's Furnace Fun quiz, then the final battle with Gruntilda and the Jinjonator

## Achievement ideas

Our own set, with our own names and descriptions. The first eight are live (`catalog/achievements/bk.json`).

### Progress (instant conditions, the engine handles them today)

- each note door opened (8)
- every jiggy of each world (10), all 100 jiggies
- all notes of each world (9), all 900
- all Jinjos of each world (9)
- every move learned; every transformation used (6)
- every warp cauldron in the lair
- all empty honeycombs and the full life bar
- all Mumbo tokens
- the three cheats and Bottles' bonus cheats
- all Stop 'n' Swop items
- 9 lives at once
- winning the quiz, the secret ending, beating Gruntilda

### Harder ones (need the engine to follow a run over time)

- beat Gruntilda without taking damage
- finish the game without extending the life bar
- a world's 1-ups in a single life
- Clanker released without surfacing or touching a bubble
- Mr. Vile's second game without the Turbo Trainers

These need conditions that reset when something happens (a life lost, damage taken, surfacing), like the "reset if" and "hit count" rules of achievement engines. The engine checks instant states only, so they come after that feature.

### Addresses still to find in the decomp

Note totals per world, Jinjo and honeycomb flags, Mumbo token flags, learned moves (`player_isAbilityUnlocked`), the current transformation, lives, health and air, warp cauldrons, cheats, Stop 'n' Swop, and the story flags (note doors, quiz, endings).
