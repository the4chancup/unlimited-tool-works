# Unlimited Tool Works — Library crates plan: fox2

Part of the [Library crates plan](README.md). Section headings are unchanged from the single-file plan, so an existing pointer to a section still names it; only the file part of the pointer changed.

## `libs/fox2`: Fox Engine entity files

A `.fox2` is the Fox Engine's serialized entity graph (a stadium's models, lights, audience and
staff placements), shipped inside FPKD archives. The reference is a Python port of the C# FoxTool; its XML is the interchange
format the stadium exports already carry (`*.fox2.xml`), so the crate speaks both. Layout
measured on the 87 distinct `.fox2` inside the 108 FPKDs on the writing machine (Konami PES 16
stadium, audience, staff, light and boots files); the fixtures under
`crates/libs/fox2/tests/fixtures/` are four of them covering every data type and container that
occurs. Everything little-endian.

- **Header, 32 bytes**: `u32 0x786F62F2`, `u32 0x35`, `i32 entity_count`,
  `i32 string_table_offset`, `i32 32` (header size), 12 zero bytes.
- **Entities**, back to back from offset 32. Each has a 64-byte header: `i16 64` (header size),
  `i16 unknown1` (values 64 to 560 in steps that look like class sizes; carried), `i16 0`,
  `u32 0x746E65` (`ent`), `u32 address` (an in-engine pointer, unique per entity, referenced by
  `EntityPtr`/`EntityHandle` values), `u32 0`, `i32 unknown2` (0 in all 6957 entities; carried),
  `i32 0`, `i16 version`, `u64 class_name_hash`, `u16 static_count`, `u16 dynamic_count`,
  `i32 64` (offset), `i32 static_data_size` (entity start to the end of the static properties),
  `i32 data_size` (entity start to the end of the dynamic ones), zero-padded from 52 to 64. Then
  the static properties, then the dynamic ones (no file on the machine has any).
- **Property**: a 32-byte header `u64 name_hash, u8 data_type, u8 container, u16 count,
  u16 32, u16 size` (the whole property, header and padding included), 16 zero bytes; then the
  values, then zero padding to 16. A `StringMap` (container 2) stores each entry as
  `u64 key_hash`, the value, padding to 16; `StaticArray` 0, `DynamicArray` 1 and `List` 3 store
  the values back to back.
- **Data types** (word, size in bytes): int8 0 (1), uint8 1 (1), int16 2 (2), uint16 3 (2),
  int32 4 (4), uint32 5 (4), int64 6 (8), uint64 7 (8), float 8 (4), double 9 (8), bool 10 (1),
  String 11, Path 12 and FilePtr 20 (8: a string hash), EntityPtr 13 and EntityHandle 21 (8),
  Vector3 14, Vector4 15, Quat 16 and Color 19 (16: four `f32`, Vector3 included), Matrix3 17
  (36), Matrix4 18 (64), EntityLink 22 (32: package, archive and name hashes then a `u64`
  handle), WideVector3 24 (16: three `f32`, two `u16`). PropertyInfo 23 has no known layout and
  is a read error. Seen on the machine: float, EntityHandle, Vector3, bool, EntityPtr, int32,
  EntityLink, uint32, String, Quat, Color, uint8, FilePtr, Path, Vector4; the rest are
  implemented from the reference and untested against a real file, and the doc says so.
- **Strings are hashes**: `hash_string(text)` is CityHash64WithSeeds over `text + NUL` with
  `seed0 = 0x9AE16A3B2F90404F` and `seed1 = (first_byte << 16) + len(text)`, masked to 48 bits;
  the CityHash is the 1.0.3 variant the C# tool embeds (`HashLen0to16`, `17to32`, `33to64`,
  `above64`), ported directly, not taken from a crate whose version cannot be pinned to that
  variant. Verified by `hash_golden.tsv` (reference hashes for 53 strings across every length
  class, the empty string and non-ASCII, the long ones non-repetitive) and by every fixture's own
  string table. Where the reference is not CityHash, we follow CityHash: its 33-64 byte branch
  leaves one sum (`fetch(16) + fetch(len - 32)`) unmasked before a rotate, so for text whose
  fetched words have the top bit set (non-ASCII only; ASCII words cannot overflow the sum) its
  hash differs from the C# tool's and so from the game's. The port wraps, as C# does; two golden
  rows hold the wrapping values.
- **String table** at `string_table_offset`: entries `u64 hash, u32 len, bytes` until a zero
  hash; then zero padding to 16, the five bytes `00 00 'e' 'n' 'd'`, zero padding to 16, end of
  file. Konami's table holds every string the file hashes except the empty string (hash
  `0xB8A0BF169F98`, which occurs as a value and which the reference resolves only through its
  dictionary's empty first line); its order varies per file (byte-sorted in some, traversal
  order in others). The compiler writes the table in entity-traversal order (class name,
  property names, keys and values as met), deduplicated on hash and text together (two literals
  that collide in 48 bits both get an entry), empty literals skipped, which is the reference's
  order and what `*.compiled.fox2` holds.
- **Reference quirk not reproduced**: the Python writer returns its over-allocated buffer, so
  its output carries hundreds of trailing zero bytes past the `end` trailer (896 bytes of content
  in a 1228-byte result). FoxTool's and Konami's files end at the aligned trailer; ours do too,
  and the compiled goldens are the reference output cut there.

Types, copied by the implementation:

```rust
pub struct Fox2File { pub entities: Vec<Entity>, pub string_table: Vec<TableEntry> }
pub struct TableEntry { pub hash: u64, pub text: String }
pub struct Entity {
    pub class_name: FoxString, pub unknown1: i16, pub unknown2: i32, pub version: i16,
    pub address: u32, pub static_properties: Vec<Property>, pub dynamic_properties: Vec<Property>,
}
pub struct Property { pub name: FoxString, pub container: Container, pub keys: Vec<FoxString>, pub values: Values }
pub enum FoxString { Literal(String), Hash(u64) }   // `hash()` hashes a literal, returns a hash as is
pub enum Container { StaticArray, DynamicArray, StringMap, List }   // words 0-3 through `word()`/`from_word()`, no `as` cast
pub enum Values {                                    // one variant per data type, `data_type()` gives the word
    Int8(Vec<i8>), Uint8(Vec<u8>), Int16(Vec<i16>), Uint16(Vec<u16>), Int32(Vec<i32>), Uint32(Vec<u32>),
    Int64(Vec<i64>), Uint64(Vec<u64>), Float(Vec<f32>), Double(Vec<f64>), Bool(Vec<bool>),
    String(Vec<FoxString>), Path(Vec<FoxString>), FilePtr(Vec<FoxString>),
    EntityPtr(Vec<u64>), EntityHandle(Vec<u64>),
    Vector3(Vec<[f32; 4]>), Vector4(Vec<[f32; 4]>), Quat(Vec<[f32; 4]>), Color(Vec<[f32; 4]>),
    Matrix3(Vec<[f32; 9]>), Matrix4(Vec<[f32; 16]>), EntityLink(Vec<EntityLink>), WideVector3(Vec<WideVector3>),
}
pub struct EntityLink { pub package: FoxString, pub archive: FoxString, pub name: FoxString, pub handle: u64 }
pub struct WideVector3 { pub x: f32, pub y: f32, pub z: f32, pub a: u16, pub b: u16 }
pub struct Dictionary(HashMap<u64, String>);         // `from_lines(text)`: one literal per line, the empty line included
```

`Fox2File::read(&[u8])` keeps every string as `Hash` and the table as read, so `write()` is
byte-identical on every Konami file (the entity region and the table alike). `resolve(&mut self,
dictionary: Option<&Dictionary>)` turns hashes into `Literal`s through the file's own table
first, then the dictionary, leaving unknown hashes as they are. A table entry whose stored hash
is not the hash of its text (the reference calls these encrypted) is skipped, as the reference
skips it: resolving through it would print a text that recompiles to a different hash.
`write()` refuses what it cannot encode faithfully rather than truncating it: a count or size
past its field's width, `StringMap` keys that are not one per value (or keys on another
container, which the layout has no room for), and a table entry with hash 0, which reads back
as the table's terminator. `to_xml(&self) -> String` writes
the FoxTool layout: two-space indent, `<fox formatVersion="2" fileVersion="0"
originalVersion="">`, a `<classes>` list of `Entity`, `Data`, then each class as first met with
its entity's version, `<entities>` (self-closing when there are none) with `class classVersion addr unknown1 unknown2`,
`<staticProperties>`/`<dynamicProperties>` holding `<property name type container arraySize>`
and `<value>` children; a `Literal` is element text (`<value></value>` for the empty string), a
`Hash` the attribute `hash="0x%08X"`; integers decimal, floats in C#'s round-trip text (the
shortest of 7 and 9 significant digits that reads back to the same `f32`, `E+NN`/`E-NN`
exponents, `-0`; `float_golden.tsv` holds 36 reference cases), doubles as the reference writes
them, which is Python's `repr` and not C#'s round-trip form (the shortest text that reads back,
lowercase `e+NN`/`e-NN` outside `1e-4 <= |x| < 1e16`, `.0` on an integral value, `-0.0`,
`nan`/`inf`/`-inf`; ties round to even, and below a power of two the shortest text may be the
one above the nearest; `double_golden.tsv` holds 30 cases; no `.fox2` on the machine has a double),
bools `true`/`false`, pointers, handles and `addr` `0x%08X`, Vector3/Vector4/Quat as `x y z w`
attributes, Color `r g b a`, Matrix3/4 as `<RowN ColumnM="..">` children, EntityLink as
`packagePath archivePath nameInArchive` attributes (`...Hash` when unresolved) with the handle
as text, WideVector3 `x y z a b`. Attribute escaping `& " <` plus tab, LF and CR as `&#9;`
`&#10;` `&#13;` (an XML reader normalizes raw ones to spaces, which would change the string and
so its hash); text escaping `& < >` and CR. Where the reference's XML is lossy, ours is not: an
unresolved class or property name prints as the reference's `class=""`/`name=""` *and* a
`classHash`/`nameHash` attribute with the hash, which `from_xml` prefers when present, so a
Konami file whose names the dictionary lacks survives a decompile/compile round trip instead of
being recompiled with the empty-string hash (the reference's behavior, which the Stadium
compiler's ID rewrite would otherwise inherit). A `StringMap` key literal is ambiguous with a
hash when it starts with `0x`; the format has no way around that, no file on the machine has
one, and `from_xml` reads `0x` followed by hex as a hash and anything else as a literal.
`Fox2File::from_xml(&str)` reads that layout with `roxmltree` (missing attributes default as the
reference's do: `0`, `""`, container `StaticArray`; blank text where the reference parses
strictly is an error, as its parse is: a present but blank float attribute, `classVersion`,
`unknown1`, `unknown2`, WideVector3 `a` or `b`, and a float or double value whose text is only
whitespace (a value
with no text is `0`; blank `addr` and blank integer values read `0`, as the reference's
lenient integer parse does); a `bool` must read `true`, `false` or empty,
anything else is an error rather than the reference's silent `false`; a value's text is all its
text children with comments dropped; float text is parsed as a double and narrowed to `f32`, the
reference's double rounding, so a 17-digit text compiles to the reference's bits; a finite text
past `f32::MAX` is an error, as the reference's packing of it is, while `inf` reads as infinity)
and builds the
string table in traversal order. The binary reader validates every padding span it skips (property tails,
`StringMap` entries, the trailer) as zero, since a rewrite zero-fills them; measured true on all
87 files, whose trailers also all end exactly at the file end. Zero bytes past the aligned
trailer are accepted (the reference writer's slack, above) and dropped on rewrite; any nonzero
byte there is an error. The XML text produced by decompiling each fixture without a dictionary equals the
reference's (`*.fox2.xml`), with a one-line dictionary (`audi_low_parts.dict.fox2.xml`) too; the
binary compiled from each equals `*.compiled.fox2`. The dictionary file itself (1.7 MB, 21543
lines) is the Stadium compiler's asset, placed when that tool is built; the lib only loads one.
