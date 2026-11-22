# Unlimited Tool Works: Library crates plan: usm

> **Plan:** work not built yet (the open phase's sections may be partly built). What the suite does now is described in `docs/spec/`.

## `libs/usm`

CRI's video files, the USMs (Sofdec2), which PES plays for the menus' wipes and movies and for
the stadium billboards. The crate writes one from encoded MPEG-1 frames, for the [Video
encoder](../video_encoder.md), and reads one's streams and properties, for the PES file
explorer and for the Video encoder's player. No phase holds it yet.

### The format, as observed

Observed in every USM the cup ships (2026-11-21): the 11 season swipes in
`C:/Data/4cc/Swipes/`, the 6 competition wipes and 2 movies of Red's Winter26 patch for
PES 2017 (`common/menu/fade/`, `movie/`), and 2 billboards in `C:/Data/4cc/0Livecpk/`
(`bill/mlp/movie/`). What a field means is inferred from its name where marked.

**Chunks.** A USM is a row of chunks. Each opens with a 32-byte header, big-endian after
the signature:

| Offset | Size | Field |
|---|---|---|
| 0 | 4 | signature: `CRID`, `@SFV` (video), `@ALP` (transparency), `@SFA` (audio) |
| 4 | 4 | payload size: every byte after this field |
| 8 | 2 | header size, always `0x18` (counted from offset 8) |
| 10 | 2 | padding size: zero bytes after the data |
| 12 | 4 | chunk type (below) |
| 16 | 4 | presentation time |
| 20 | 2 | reserved |
| 22 | 2 | `0x1e` on the `CRID` and metadata chunks, `0x1770` on a frame chunk (not decoded) |
| 24 | 4 | metadata offset |
| 28 | 4 | metadata size |

The chunk's content starts at offset `8 + header size`: an `@UTF` table (the same CRI table
format CPKs use, which `libs/cpk` reads) on the `CRID` and metadata chunks, the stream's
bytes on a frame chunk.

**Chunk types.** `0` is one frame of a stream. `1` is the stream's header: an `@UTF` table
named `VIDEO_HDRINFO` (width, height, `mpeg_codec`, `alpha_type`, `total_frames`,
`framerate_n`/`framerate_d`, and sizes and offsets the player uses) or `AUDIO_HDRINFO`
(`audio_codec`, `sampling_rate`, `total_samples`, `num_channels`). `2` is a 32-byte
`#HEADER END` or `#FOOTER END` marker, three per stream. `3` is the stream's index, a
second metadata chunk.

**The file.** The first chunk is always `CRID`, its payload `0x7f8` bytes in every file: an
`@UTF` table named `CRIUSF_DIR_STREAM` with one row for the file (`stmid` 0) and one per
stream (`stmid` the stream's signature read as a u32), its columns `fmtver`, `filename`,
`filesize`, `datasize`, `stmid`, `chno`, `minchk`, `minbuf` and `avbps` (inferred: the
minimum chunk count, the minimum buffer and the average bit rate). The streams' chunks
follow, interleaved.

**The streams.**

- Video: MPEG-1 in every file (`mpeg_codec` 1); its first frame opens with an MPEG-1
  sequence header (`00 00 01 B3`) and has no MPEG-2 extension. The frames carry TMPGEnc's
  user data (`TMPGEXS`), which the encoder's library writes; the game needing it is
  unlikely (inferred).
- Transparency: a second MPEG-1 stream, `@ALP` (`alpha_type` 1), at the video's size and
  frame rate, holding the matte. Every swipe, wipe and the 720x480 billboard carry one; the
  two movies and the 1024x64 billboard do not.
- Audio: only the two movies have it, `@SFA` with `audio_codec` 2, stereo at 48,000 or
  44,100 Hz. Codec 2 is CRI's HCA (inferred: the encoder's batch files only ever ask for
  HCA), which has no open encoder.

**Sizes and rates.** The swipes and wipes are 1280x720 or 1920x1080, at 25 to 60 frames a
second, and last about 2 seconds. All six Winter26 wipes are byte for byte the same file as `Swipes/Clover.usm`:
the game plays each wipe by its file name, so a member drops one file in under each.

**How they are made today.** Bent's 2015 batch files (`movies/*.bat` in
`C:/Data/4cc/Tools_4cc/4CC_PES15_tool_compilation_v0.1_alpha/`) convert the source with
ffmpeg to MPEG-4 in AVI and the sound to PCM WAV, then run CRI's `medianoche.exe` with a
closed GOP (`-gop_closed=on -gop_i=1 -gop_p=4 -gop_b=2`), 16 Mbit/s for 720p
(`-scale=1280,720`) or 8 Mbit/s unscaled, `-alpha00` for the transparency stream from the
same source, and `-hca=on -hca_quality=4` for sound.

### What the writer needs

- The container: the `CRID` table, each stream's header, frame, marker and index chunks.
  The fields marked inferred above are checked by writing a file the game plays, not
  assumed.
- MPEG-1 frames for the video and the matte, from any MPEG-1 encoder (ffmpeg's
  `mpeg1video` is one; which one is a dependency decision, made when a phase opens).
- No sound at first: no wipe, swipe or billboard has any, and HCA has no open encoder.
- Whether the game accepts other GOP, buffer or index values than `medianoche`'s is not
  known: the first target is the values these files carry.
