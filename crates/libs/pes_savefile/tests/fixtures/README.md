# pes_savefile fixtures

Slices of real saves, never a whole one: a save is 5-11 MB and incompressible once encrypted,
and its serial section is the writing account's Windows SID (plan: `spec/pes_savefile/container.md` "Container fixtures"). Extracted by `scripts/provenance/fixtures/save_fixtures.py` from the saves below.
Konami-derived data is Konami's, kept for interoperability (see the repository README).

Per version `NN`:

| File | Holds |
|---|---|
| `pesNN_head.bin` | The file prefix through the description: salt (320) + encrypted header (176 on 16/17, 208 on 18-21) + encrypted description (384); on 15 the seed byte + three MD5 digests + encrypted description (384) + the logo length (4) |
| `pesNN_payload_enc_head.bin` | The first 4096 encrypted bytes of the payload section |
| `pesNN_payload.bin.zz` | The whole decrypted payload, zlib level 9; tests inflate it |

| Version | Source save | Payload / logo / serial (bytes, UTF-16 units) | Notes |
|---|---|---|---|
| 15 | `Tools_4cc/Midcupping/EDIT.bin` (a 4cc PES 15 cup save; identical to the PES 15 KONAMI save on the machine) | 5782808 / 28795 / - | seed byte 195; payload MD5 `c8d4d78bf43317b02b66786b92ce2d0c` equals the head's third digest |
| 16 | `Documents/KONAMI/Pro Evolution Soccer 2016/save/EDIT00000000` | 5886176 / 67218 / 46 | header 176; identifier ends `EDIT` + zeros |
| 17 | `Documents/KONAMI/Pro Evolution Soccer 2017/save/EDIT00000000` | 5266180 / 28689 / 46 | header 176 |
| 18 | `Documents/KONAMI/PRO EVOLUTION SOCCER 2018/save/EDIT00000000` | 5203344 / 66179 / 45 | header 208; game version string `PRO EVOLUTION SOCCER 2018` |
| 19 | `Documents/KONAMI/PRO EVOLUTION SOCCER 2019/<account>/save/EDIT00000000` (identical to `Cups/Genso Cup 2/EDIT00000000`) | 7350036 / 27504 / 46 | header 208; `PRO EVOLUTION SOCCER 2019` |
| 20 | `Documents/KONAMI/eFootball PES 2020/<account>/save/EDIT00000000-day0` (a 4cc invitational's day-0 save: 414 named players on two-ish teams, the other 4646 `PLACEHOLDER`; the sibling `EDIT00000000` is the fresh game-generated save with every player a placeholder, which some golden non-vacuity floors cannot meet) | 10964500 / 13951 / 44 | header 208; `eFootball PES 2020` |
| 21 | `Documents/KONAMI/eFootball PES 2021 SEASON UPDATE/<account>/save/EDIT00000000` | 10995800 / 14235 / 45 | header 208; `eFootball PES 2021 SEASON UPDATE` |

## Interchange fixtures (2.17h)

Extracted by `scripts/provenance/fixtures/interchange_fixtures.py`, which also prints the literals
the golden tests hold.

| File | Holds |
|---|---|
| `pes18_texport.ted`, `pes19_texport.ted`, `pes20_texport.ted`, `pes21_texport.ted` | Whole real Texport files (8128 / 9648 / 14760 / 14820 bytes; no personal data beyond the team's content): `WEPES/osyearduh.ted` (PES 18, team 736 `/o/`), `WEPES/98HU.ted` (PES 19, team 841 `/98hu/`), `WEPES/team3.ted` (PES 20, team 701 `/3/` of the cup's base DLC on `F:/Games/PES2020`, every player `PLACEHOLDER`; exported by the lead through the in-game harness on 2026-10-12), `WEPES/98hu_vrl4_v1.ted` (PES 21, team 841). 23 players each |
| `pes17_texport_head.bin`, `pes17_texport_payload_enc_head.bin`, `pes17_texport_payload.bin.zz` | The save-style slices of `Saves/pesXcrypter/TEXPORT00000000`, a PES 17 texport (team 736, 23 players): container header 176, payload 5578704, logo 31590, description `Team Export Data 03` with the team name `/o/` at 0x80, serial 46 units; payload MD5 `d7a58b1b07efd13fb9861fa5fd53a6a6` |
| `pes16_texport_head.bin`, `pes16_texport_payload_enc_head.bin`, `pes16_texport_payload.bin.zz` | The same slices of `Documents/KONAMI/Pro Evolution Soccer 2016/save old2/TEXPORT00000001`, a PES 16 texport (team 804, `VGL 4`, 23 players whose roster, and so whose records, are out of id order: 80401, 80402, 80423, 80404, ...): container header 176, payload 5578096, logo 69592, description `Team Export Data 02`, serial 46 units; payload MD5 `8bced48807770ea987e5bb9199290265` (added at the Phase 5 converge) |
| `pes19_squad.4ccs` | A real `.4ccs` (`PRO EVOLUTION SOCCER 2019/<account>/save/CM.4ccs`, team 713, 23 players, tag `20a`, no tactics block) |
| `pesNN_parity.4ccs` (NN = 15 to 21) | Real `.4ccs` files exported by the maintainer with 4ccEditor on 2026-09-30 from the saves the payload fixtures above hold (each save's decrypted payload has the fixture's SHA-256), one team each: `/a/` (team 702) on 15 to 19, team 857 on 20, team 782 on 21; 23 players, no tactics block. The PES 18 file is a second export from a freshly opened editor: the first repeated player 1, dropped player 23 and had 22 players' visual edit flags cleared. The parity evidence of `verification.md` "Cross-implementation parity" |
| `pes19_tactics.4cct` | **Synthesized**: the reference editor's `save_tactical_data` write walk (transcribed in the script) over team 713's tactics record of the PES 19 payload fixture, header `001` + `19` + `713`; block MD5 `4985ec743656067c618235ef11e9a2a3`. No real `.4cct` exists on the machine |

## The overall formula's fixture

| File | Holds |
|---|---|
| `overall_points.tsv` | 351 overalls read in game off the formation screens (the badge's number, a held player's swap preview, the drag panel's): 150 in PES 2017 on team /3/ (701) of the maintainer's PES 2017 save, 201 in PES 2021 on teams 717 and 718 of the PES 2021 save, all read 2026-10-27/28 with the pad and OBS. Each row: the version, team and roster key, the player's abilities, registered position and playable ratings (letters in the position byte's order, GK to CF) as his team's Team TOML export gives them, the position, the overall `shown` and the overall PES 2016's formula `computed` (the lead's script over the same exports, its tables checked entry for entry against the executable's). Every PES 2017 row has `computed` equal to `shown`; PES 2021's are within 2 |
