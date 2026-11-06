//! Resolving a material for an FMDL export: the family's Fox defaults, then the
//! engine-neutral booleans on the bits they own, then the verbatim `fox` table.

use super::{Material, MaterialFamily, TextureRole};

/// Alpha flag bit for two-sided rendering; owned by `two_sided` when that is set explicitly.
const TWO_SIDED_BIT: u8 = 32;
/// Alpha flag bit for transparency; owned by `transparent` when that is set explicitly.
const TRANSPARENT_BIT: u8 = 128;
/// Shadow flag bit for "does not cast a shadow"; owned by `cast_shadow` (true clears it).
pub(crate) const NO_SHADOW_CAST_BIT: u8 = 1;
/// Shadow flag bit for "renders nothing"; owned by `invisible`.
pub(crate) const INVISIBLE_BIT: u8 = 2;

/// The family's Fox defaults (format plan "Shader families", Fox columns).
pub struct FoxDefaults {
    /// The default FMDL shader name.
    pub shader: &'static str,
    /// The default FMDL technique name.
    pub technique: &'static str,
    /// The default alpha flags.
    pub alpha_flags: u8,
    /// The default shadow flags.
    pub shadow_flags: u8,
    /// The default shader parameters.
    pub parameters: &'static [(&'static str, [f32; 4])],
}

/// The Fox defaults of each family.
pub fn defaults(family: MaterialFamily) -> FoxDefaults {
    match family {
        MaterialFamily::Shaded => FoxDefaults {
            shader: "fox3ddf_blin",
            technique: "fox3DDF_Blin",
            alpha_flags: 128,
            shadow_flags: 0,
            parameters: &[("MatParamIndex_0", [0.0, 0.0, 0.0, 0.0])],
        },
        MaterialFamily::Shadeless => FoxDefaults {
            shader: "fox3dfw_constant_srgb_ndr_solid",
            technique: "fox3DFW_ConstantSRGB_NDR_Solid",
            alpha_flags: 16,
            shadow_flags: 5,
            parameters: &[],
        },
        MaterialFamily::Metal => FoxDefaults {
            shader: "fox3ddf_ggx",
            technique: "fox3DDF_GGX",
            alpha_flags: 128,
            shadow_flags: 0,
            parameters: &[("MatParamIndex_0", [0.0, 0.0, 0.0, 0.0])],
        },
        MaterialFamily::Glass => FoxDefaults {
            shader: "pes3dfw_glass2",
            technique: "pes3DFW_Glass2",
            alpha_flags: 16,
            shadow_flags: 5,
            parameters: &[
                ("MatParamIndex_0", [54.0, 0.0, 0.0, 0.0]),
                ("ReflectionIntensity", [1.0, 0.0, 0.0, 0.0]),
                ("GlassRoughness", [0.0, 0.0, 0.0, 0.0]),
                ("GlassFlatness", [0.0, 0.0, 0.0, 0.0]),
                ("PCBoxCenter", [0.0, 15.0, 0.0, 0.0]),
                ("PCBoxSize", [250.0, 80.0, 250.0, 0.0]),
            ],
        },
    }
}

/// The Fox sampler a canonical role binds to; `None` for roles Fox has no sampler for.
pub fn sampler_for_role(role: TextureRole, base_linear: bool) -> Option<&'static str> {
    match role {
        TextureRole::Base => Some(if base_linear {
            "Base_Tex_LIN"
        } else {
            "Base_Tex_SRGB"
        }),
        TextureRole::Normal => Some("NormalMap_Tex_NRM"),
        TextureRole::Specular => Some("SpecularMap_Tex_LIN"),
        TextureRole::Metalness => Some("MetalnessMap_Tex_LIN"),
        TextureRole::Reflection => Some("GlassReflection_Tex_SRGB"),
        TextureRole::ReflectionMask => Some("GlassReflectionMask_Tex_LIN"),
        TextureRole::Environment | TextureRole::DetailMaterial | TextureRole::DetailNormal => None,
    }
}

/// The inverse of [`sampler_for_role`]: `None` for any name that is not a canonical role's
/// Fox sampler (it stays a native sampler).
pub fn role_for_sampler(sampler: &str) -> Option<TextureRole> {
    match sampler {
        "Base_Tex_SRGB" | "Base_Tex_LIN" => Some(TextureRole::Base),
        "NormalMap_Tex_NRM" => Some(TextureRole::Normal),
        "SpecularMap_Tex_LIN" => Some(TextureRole::Specular),
        "MetalnessMap_Tex_LIN" => Some(TextureRole::Metalness),
        "GlassReflection_Tex_SRGB" => Some(TextureRole::Reflection),
        "GlassReflectionMask_Tex_LIN" => Some(TextureRole::ReflectionMask),
        _ => None,
    }
}

/// What an FMDL exporter writes for `material`: the `fox` table when present, else the
/// family defaults; then the engine-neutral booleans, then the parameter merge.
#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedFox {
    /// The shader name.
    pub shader: String,
    /// The technique name.
    pub technique: String,
    /// The resolved alpha flags.
    pub alpha_flags: u8,
    /// The resolved shadow flags.
    pub shadow_flags: u8,
    /// `(sampler name, texture index)`: canonical roles first in `TextureRole` order through
    /// `sampler_for_role`, then `fox.textures` verbatim.
    pub textures: Vec<(String, usize)>,
    /// Canonical roles Fox has no sampler for (the caller reports `material_texture_unused`).
    pub unused_roles: Vec<TextureRole>,
    /// The family defaults (only without a `fox` table) < `material.parameters` <
    /// `fox.parameters`; a repeated name keeps its position and takes the later value.
    pub parameters: Vec<(String, [f32; 4])>,
}

/// Merges `(name, value)` pairs in order: a name already present keeps its position and
/// takes the new value.
fn merge(into: &mut Vec<(String, [f32; 4])>, name: &str, value: [f32; 4]) {
    match into.iter_mut().find(|(existing, _)| existing == name) {
        Some((_, slot)) => *slot = value,
        None => into.push((name.to_string(), value)),
    }
}

/// Resolves `material` for an FMDL export.
pub fn resolve(material: &Material) -> ResolvedFox {
    let defaults = defaults(material.family);
    let fox = material.fox.as_ref();

    let mut alpha_flags = fox.map_or(defaults.alpha_flags, |f| f.alpha_flags);
    if let Some(two_sided) = material.two_sided {
        alpha_flags = if two_sided {
            alpha_flags | TWO_SIDED_BIT
        } else {
            alpha_flags & !TWO_SIDED_BIT
        };
    }
    if let Some(transparent) = material.transparent {
        alpha_flags = if transparent {
            alpha_flags | TRANSPARENT_BIT
        } else {
            alpha_flags & !TRANSPARENT_BIT
        };
    }
    let mut shadow_flags = fox.map_or(defaults.shadow_flags, |f| f.shadow_flags);
    if let Some(cast_shadow) = fox.and_then(|f| f.cast_shadow) {
        shadow_flags = if cast_shadow {
            shadow_flags & !NO_SHADOW_CAST_BIT
        } else {
            shadow_flags | NO_SHADOW_CAST_BIT
        };
    }
    if let Some(invisible) = fox.and_then(|f| f.invisible) {
        shadow_flags = if invisible {
            shadow_flags | INVISIBLE_BIT
        } else {
            shadow_flags & !INVISIBLE_BIT
        };
    }

    // The base texture's colour space rides both directions: `Base_Tex_LIN`
    // sets `fox.base_linear`, a `.mtl` `DiffuseMap` with `srgb="0"` means it
    // too — the first `prefox` sampler whose name holds the Base role is the
    // one that produced the canonical binding.
    let base_linear = fox.is_some_and(|f| f.base_linear)
        || material
            .prefox
            .as_ref()
            .and_then(|prefox| {
                prefox.samplers.iter().find(|(name, _)| {
                    super::to_prefox::role_for_sampler(name) == Some(TextureRole::Base)
                })
            })
            .is_some_and(|(_, settings)| settings.srgb == Some(false));
    let mut textures = Vec::new();
    let mut unused_roles = Vec::new();
    let mut roles: Vec<(TextureRole, usize)> = material.textures.clone();
    roles.sort_by_key(|(role, _)| *role);
    for (role, texture) in roles {
        match sampler_for_role(role, base_linear) {
            Some(sampler) => textures.push((sampler.to_string(), texture)),
            None => unused_roles.push(role),
        }
    }
    if let Some(fox) = fox {
        for (sampler, texture) in &fox.textures {
            textures.push((sampler.clone(), *texture));
        }
    }

    let mut parameters = Vec::new();
    if fox.is_none() {
        for (name, value) in defaults.parameters {
            merge(&mut parameters, name, *value);
        }
    }
    for (name, value) in &material.parameters {
        merge(&mut parameters, name, *value);
    }
    if let Some(fox) = fox {
        for (name, value) in &fox.parameters {
            merge(&mut parameters, name, *value);
        }
    }

    ResolvedFox {
        shader: fox.map_or_else(|| defaults.shader.to_string(), |f| f.shader.clone()),
        technique: fox.map_or_else(|| defaults.technique.to_string(), |f| f.technique.clone()),
        alpha_flags,
        shadow_flags,
        textures,
        unused_roles,
        parameters,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::materials::{FoxMaterial, PreFoxMaterial, SamplerSettings};

    fn material(family: MaterialFamily) -> Material {
        Material {
            name: "mat".to_string(),
            family,
            two_sided: None,
            transparent: None,
            textures: vec![],
            parameters: vec![],
            fox: None,
            prefox: None,
        }
    }

    #[test]
    fn family_defaults() {
        let shaded = defaults(MaterialFamily::Shaded);
        assert_eq!(shaded.shader, "fox3ddf_blin");
        assert_eq!(shaded.technique, "fox3DDF_Blin");
        assert_eq!(shaded.alpha_flags, 128);
        assert_eq!(shaded.shadow_flags, 0);
        assert_eq!(
            shaded.parameters,
            [("MatParamIndex_0", [0.0, 0.0, 0.0, 0.0])]
        );
        let shadeless = defaults(MaterialFamily::Shadeless);
        assert_eq!(shadeless.shader, "fox3dfw_constant_srgb_ndr_solid");
        assert_eq!(shadeless.technique, "fox3DFW_ConstantSRGB_NDR_Solid");
        assert_eq!(shadeless.alpha_flags, 16);
        assert_eq!(shadeless.shadow_flags, 5);
        assert!(shadeless.parameters.is_empty());
        let metal = defaults(MaterialFamily::Metal);
        assert_eq!(metal.shader, "fox3ddf_ggx");
        assert_eq!(metal.technique, "fox3DDF_GGX");
        assert_eq!(metal.alpha_flags, 128);
        assert_eq!(metal.shadow_flags, 0);
        assert_eq!(
            metal.parameters,
            [("MatParamIndex_0", [0.0, 0.0, 0.0, 0.0])]
        );
        let glass = defaults(MaterialFamily::Glass);
        assert_eq!(glass.shader, "pes3dfw_glass2");
        assert_eq!(glass.technique, "pes3DFW_Glass2");
        assert_eq!(glass.alpha_flags, 16);
        assert_eq!(glass.shadow_flags, 5);
        assert_eq!(
            glass.parameters,
            [
                ("MatParamIndex_0", [54.0, 0.0, 0.0, 0.0]),
                ("ReflectionIntensity", [1.0, 0.0, 0.0, 0.0]),
                ("GlassRoughness", [0.0, 0.0, 0.0, 0.0]),
                ("GlassFlatness", [0.0, 0.0, 0.0, 0.0]),
                ("PCBoxCenter", [0.0, 15.0, 0.0, 0.0]),
                ("PCBoxSize", [250.0, 80.0, 250.0, 0.0]),
            ]
        );
    }

    #[test]
    fn role_and_sampler_names_are_inverses() {
        for role in [
            TextureRole::Base,
            TextureRole::Normal,
            TextureRole::Specular,
            TextureRole::Metalness,
            TextureRole::Reflection,
            TextureRole::ReflectionMask,
        ] {
            let sampler = sampler_for_role(role, false).expect("sampler");
            assert_eq!(role_for_sampler(sampler), Some(role));
        }
        assert_eq!(
            sampler_for_role(TextureRole::Base, true),
            Some("Base_Tex_LIN")
        );
        assert_eq!(role_for_sampler("Base_Tex_LIN"), Some(TextureRole::Base));
        for role in [
            TextureRole::Environment,
            TextureRole::DetailMaterial,
            TextureRole::DetailNormal,
        ] {
            assert_eq!(sampler_for_role(role, false), None);
        }
        assert_eq!(role_for_sampler("RoughnessMap"), None);
    }

    #[test]
    fn resolve_applies_the_layers() {
        let resolved = resolve(&material(MaterialFamily::Shaded));
        assert_eq!(resolved.shader, "fox3ddf_blin");
        assert_eq!(resolved.technique, "fox3DDF_Blin");
        assert_eq!(resolved.alpha_flags, 128);
        assert_eq!(resolved.shadow_flags, 0);
        assert_eq!(
            resolved.parameters,
            [("MatParamIndex_0".to_string(), [0.0, 0.0, 0.0, 0.0])]
        );

        let mut two_sided = material(MaterialFamily::Shaded);
        two_sided.two_sided = Some(true);
        assert_eq!(resolve(&two_sided).alpha_flags, 160);

        let mut shadeless = material(MaterialFamily::Shadeless);
        shadeless.transparent = Some(true);
        let resolved = resolve(&shadeless);
        assert_eq!(resolved.alpha_flags, 144);

        let mut with_table = material(MaterialFamily::Shaded);
        with_table.two_sided = Some(true);
        with_table.fox = Some(FoxMaterial {
            shader: "custom".to_string(),
            technique: "custom_tech".to_string(),
            alpha_flags: 0,
            shadow_flags: 0,
            cast_shadow: Some(false),
            invisible: None,
            base_linear: false,
            textures: vec![],
            parameters: vec![],
        });
        let resolved = resolve(&with_table);
        assert_eq!(resolved.alpha_flags, 32);
        assert_eq!(resolved.shadow_flags, 1);
        assert_eq!(resolved.shader, "custom");
    }

    #[test]
    fn flag_overrides_set_and_clear_bits() {
        // TRANSPARENT_BIT 128: the Shaded default carries it; Some(false)
        // clears it, Some(true) keeps it set.
        let mut mat = material(MaterialFamily::Shaded);
        mat.transparent = Some(false);
        assert_eq!(resolve(&mat).alpha_flags, 0);
        mat.two_sided = Some(true);
        assert_eq!(resolve(&mat).alpha_flags, 32);
        mat.transparent = Some(true);
        assert_eq!(resolve(&mat).alpha_flags, 160);
        // TWO_SIDED_BIT 32 already on the table's word: Some(true) keeps it set.
        let mut mat = material(MaterialFamily::Shaded);
        mat.two_sided = Some(true);
        mat.fox = Some(FoxMaterial {
            shader: "s".to_string(),
            technique: "t".to_string(),
            alpha_flags: TWO_SIDED_BIT,
            shadow_flags: 0,
            cast_shadow: None,
            invisible: None,
            base_linear: false,
            textures: vec![],
            parameters: vec![],
        });
        assert_eq!(resolve(&mat).alpha_flags, 32);
        // NO_SHADOW_CAST_BIT 1 and INVISIBLE_BIT 2 on the table's word.
        let mut mat = material(MaterialFamily::Shaded);
        mat.fox = Some(FoxMaterial {
            shader: "s".to_string(),
            technique: "t".to_string(),
            alpha_flags: 0,
            shadow_flags: 3,
            cast_shadow: Some(true),
            invisible: Some(false),
            base_linear: false,
            textures: vec![],
            parameters: vec![],
        });
        // cast_shadow clears bit 1 (3 -> 2); invisible Some(false) clears
        // bit 2 (2 -> 0).
        assert_eq!(resolve(&mat).shadow_flags, 0);
        // cast_shadow Some(false) sets bit 1 (2 -> 3); the clear still lands.
        mat.fox.as_mut().expect("fox").cast_shadow = Some(false);
        assert_eq!(resolve(&mat).shadow_flags, 1);
        // invisible Some(true) sets bit 2 back.
        mat.fox.as_mut().expect("fox").invisible = Some(true);
        assert_eq!(resolve(&mat).shadow_flags, 3);
    }

    #[test]
    fn a_linear_prefox_base_stays_linear() {
        // A `.mtl` `DiffuseMap` with `srgb="0"` (the wideface exports' case)
        // maps to `Base_Tex_LIN`, not the sRGB default.
        let mut mat = material(MaterialFamily::Shadeless);
        mat.textures = vec![(TextureRole::Base, 0)];
        mat.prefox = Some(PreFoxMaterial {
            shader: "Shadeless".to_string(),
            states: vec![],
            samplers: vec![(
                "DiffuseMap".to_string(),
                SamplerSettings {
                    srgb: Some(false),
                    ..SamplerSettings::default()
                },
            )],
            textures: vec![],
            parameters: vec![],
        });
        assert_eq!(resolve(&mat).textures, [("Base_Tex_LIN".to_string(), 0)]);
        // `srgb="1"` keeps the sRGB sampler.
        mat.prefox.as_mut().expect("prefox").samplers[0].1.srgb = Some(true);
        assert_eq!(resolve(&mat).textures, [("Base_Tex_SRGB".to_string(), 0)]);
    }

    #[test]
    fn samplers_come_out_in_role_order() {
        // A source file that lists its normal map before its diffuse map:
        // the written samplers keep the role order, not the listing's.
        let mut mat = material(MaterialFamily::Shaded);
        mat.textures = vec![(TextureRole::Normal, 1), (TextureRole::Base, 0)];
        assert_eq!(
            resolve(&mat).textures,
            [
                ("Base_Tex_SRGB".to_string(), 0),
                ("NormalMap_Tex_NRM".to_string(), 1)
            ]
        );
    }

    #[test]
    fn a_native_table_skips_the_family_defaults() {
        let mut mat = material(MaterialFamily::Shaded);
        mat.fox = Some(FoxMaterial {
            shader: "s".to_string(),
            technique: "t".to_string(),
            alpha_flags: 0,
            shadow_flags: 0,
            cast_shadow: None,
            invisible: None,
            base_linear: false,
            textures: vec![],
            parameters: vec![],
        });
        assert!(resolve(&mat).parameters.is_empty());
    }

    #[test]
    fn parameter_merge_keeps_the_later_value() {
        let mut mat = material(MaterialFamily::Shaded);
        mat.parameters = vec![("MatParamIndex_0".to_string(), [1.0, 0.0, 0.0, 0.0])];
        mat.fox = Some(FoxMaterial {
            shader: "s".to_string(),
            technique: "t".to_string(),
            alpha_flags: 0,
            shadow_flags: 0,
            cast_shadow: None,
            invisible: None,
            base_linear: false,
            textures: vec![],
            parameters: vec![("MatParamIndex_0".to_string(), [3.0, 0.0, 0.0, 0.0])],
        });
        let resolved = resolve(&mat);
        assert_eq!(
            resolved.parameters,
            [("MatParamIndex_0".to_string(), [3.0, 0.0, 0.0, 0.0])]
        );
    }
}
