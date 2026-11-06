//! The engine-neutral material schema: what a material *is* (its shader family and canonical
//! texture roles) plus one optional verbatim table per engine.

/// Reading a shader family off a native shader name.
pub(crate) mod family;
/// Resolving a material for an FMDL export.
pub(crate) mod to_fox;
/// Resolving a material for a `.model` export.
pub(crate) mod to_prefox;

// The family rule is the one thing a caller needs before converting: the Team compiler's
// deep pass asks it which FMDL materials are metal, to plan the environment map their
// conversion names, with the same answer the conversion itself gives.
pub use family::{InferredFamily, from_fox_shader};

/// The shader family: what a material *is*, engine-neutral (format plan "Shader families").
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MaterialFamily {
    /// The standard lit material: skin, hair, cloth, boots.
    Shaded,
    /// Unlit: eyes, decals baked into the texture, emissive parts.
    Shadeless,
    /// Metallic surfaces.
    Metal,
    /// Visors and lenses.
    Glass,
}

/// A canonical texture role (format plan "Textures"); each engine maps it to a sampler name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TextureRole {
    /// The diffuse/albedo map.
    Base,
    /// The normal map.
    Normal,
    /// The specular map.
    Specular,
    /// The metalness map (Fox only).
    Metalness,
    /// The environment cubemap (pre-Fox only: the `R` in `Basic_CNSR`).
    Environment,
    /// The glass reflection cubemap (Fox only).
    Reflection,
    /// The glass reflection mask (Fox only).
    ReflectionMask,
    /// The detail material map (pre-Fox only).
    DetailMaterial,
    /// The detail normal map (pre-Fox only).
    DetailNormal,
}

/// A material: the engine-neutral core plus one optional table per engine. Mirrors the
/// `materials.toml` schema (see the Unified model format plan). An engine table is `Some`
/// when imported from that engine's format or from a toml that fills it; exporters derive a
/// missing engine table from `family` via the shader-family defaults.
#[derive(Debug, Clone, PartialEq)]
pub struct Material {
    /// The material's name (`face`, `boots`, ...); also the stem auto-detected textures
    /// are named after.
    pub name: String,
    /// What the material is; drives the engine defaults for both tables.
    pub family: MaterialFamily,
    /// Two-sided rendering; `None` defers to the engine table's raw flags.
    pub two_sided: Option<bool>,
    /// Transparency; `None` defers to the engine table's raw flags.
    pub transparent: Option<bool>,
    /// Canonical role to index into `CanonicalModel::textures`.
    pub textures: Vec<(TextureRole, usize)>,
    /// Engine-neutral shader parameters.
    pub parameters: Vec<(String, [f32; 4])>,
    /// The Fox (FMDL) side of the material, verbatim.
    pub fox: Option<FoxMaterial>,
    /// The pre-Fox (`.mtl`) side of the material, verbatim.
    pub prefox: Option<PreFoxMaterial>,
}

/// The FMDL-side fields of a material, kept verbatim on a Fox import.
#[derive(Debug, Clone, PartialEq)]
pub struct FoxMaterial {
    /// The FMDL shader name (`fox3ddf_blin`, ...).
    pub shader: String,
    /// The FMDL technique name (`fox3DDF_Blin`, ...).
    pub technique: String,
    /// Raw per-mesh alpha flags as FMDL stores them.
    pub alpha_flags: u8,
    /// Raw per-mesh shadow flags as FMDL stores them.
    pub shadow_flags: u8,
    /// Shadow flag bit 1 clear; `None` leaves the raw flags alone.
    pub cast_shadow: Option<bool>,
    /// Shadow flag bit 2 set; `None` leaves the raw flags alone.
    pub invisible: Option<bool>,
    /// Sample the base map linear (`Base_Tex_LIN`) instead of sRGB (`Base_Tex_SRGB`).
    pub base_linear: bool,
    /// Native sampler name to texture index, for samplers outside the canonical roles.
    pub textures: Vec<(String, usize)>,
    /// Native FMDL parameters, verbatim.
    pub parameters: Vec<(String, [f32; 4])>,
}

/// The `.mtl`-side fields of a material, kept verbatim on a pre-Fox import.
#[derive(Debug, Clone, PartialEq)]
pub struct PreFoxMaterial {
    /// The `.mtl` shader name (`Basic_CNSR`, `Shadeless`, ...).
    pub shader: String,
    /// The `.mtl` render states as stored (`ztest`, `zwrite`, `twosided`, `alphatest`,
    /// `alpharef`, `alphablend`, `blendmode`).
    pub states: Vec<(String, u32)>,
    /// Native sampler name to its settings; name and path live in `Texture`.
    pub samplers: Vec<(String, SamplerSettings)>,
    /// Native sampler name to index into `CanonicalModel::textures`, for samplers outside the
    /// canonical role table (`RoughnessMap`, `Normal2`, ...); canonical ones live in
    /// `Material::textures`.
    pub textures: Vec<(String, usize)>,
    /// The `.mtl` `<vector>` elements, one to four components as stored.
    pub parameters: Vec<(String, Vec<f32>)>,
}

/// The `.mtl` sampler attributes minus name and path (which live in `Texture`); the same
/// closed sets `pes_model`'s material codec reads, redeclared here because `materials/`
/// imports no format crate. `Default` is all-`None`: no attribute written.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SamplerSettings {
    /// Whether the sampler reads the texture as sRGB.
    pub srgb: Option<bool>,
    /// The minification filter.
    pub minfilter: Option<Filter>,
    /// The maxification filter (the `.mtl` schema's attribute between
    /// `minfilter` and `magfilter`).
    pub maxfilter: Option<Filter>,
    /// The magnification filter.
    pub magfilter: Option<Filter>,
    /// The mip filter.
    pub mipfilter: Option<Filter>,
    /// Addressing mode on u.
    pub uaddr: Option<Address>,
    /// Addressing mode on v.
    pub vaddr: Option<Address>,
    /// Addressing mode on w.
    pub waddr: Option<Address>,
    /// The anisotropic filtering level.
    pub maxaniso: Option<u32>,
}

/// A `.mtl` sampler filter value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Filter {
    /// Linear interpolation.
    Linear,
    /// Nearest texel.
    Point,
    /// Anisotropic filtering.
    Anisotropic,
}

/// A `.mtl` sampler addressing mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Address {
    /// Wrap (repeat the texture).
    Wrap,
    /// Clamp to the edge texel.
    Clamp,
    /// Repeat, as the `.mtl` attribute spells it.
    Repeat,
}
