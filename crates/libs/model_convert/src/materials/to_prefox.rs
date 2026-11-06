//! Resolving a material for a `.model` export: the family's `.mtl` shader and state set, the
//! engine-neutral booleans on the states they own, then the verbatim `prefox` table.

use super::{Address, Filter, Material, MaterialFamily, SamplerSettings, TextureRole};

/// The seven states in the order every output `.mtl` writes them.
pub const STATE_ORDER: [&str; 7] = [
    "ztest",
    "zwrite",
    "twosided",
    "alphatest",
    "alpharef",
    "alphablend",
    "blendmode",
];

/// The opaque set `[1, 1, 0, 1, 0, 0, 0]` or the transparent set `[1, 0, 0, 0, 0, 1, 0]`, in
/// `STATE_ORDER`.
pub fn state_set(transparent: bool) -> [(&'static str, u32); 7] {
    let (zwrite, alphatest, alphablend) = if transparent { (0, 0, 1) } else { (1, 1, 0) };
    [
        ("ztest", 1),
        ("zwrite", zwrite),
        ("twosided", 0),
        ("alphatest", alphatest),
        ("alpharef", 0),
        ("alphablend", alphablend),
        ("blendmode", 0),
    ]
}

/// The `shaded` ladder: the shortest `Basic_*` rung covering the roles present: `Basic_C`,
/// `Basic_CN` (Normal), `Basic_CNS` (Normal + Specular), `Basic_CNSR` (+ Environment). A
/// Specular without Normal still needs `Basic_CNS` (rungs tolerate a missing lower map).
pub fn shaded_shader(roles: &[TextureRole]) -> &'static str {
    let has = |role| roles.contains(&role);
    if has(TextureRole::Environment) {
        "Basic_CNSR"
    } else if has(TextureRole::Specular) {
        "Basic_CNS"
    } else if has(TextureRole::Normal) {
        "Basic_CN"
    } else {
        "Basic_C"
    }
}

/// `Shadeless` → `"Shadeless"`, `Metal` → `"Basic_CNSR"`, `Glass` → `"Basic_C"`, `Shaded` →
/// the ladder.
pub fn default_shader(family: MaterialFamily, roles: &[TextureRole]) -> &'static str {
    match family {
        MaterialFamily::Shadeless => "Shadeless",
        MaterialFamily::Metal => "Basic_CNSR",
        MaterialFamily::Glass => "Basic_C",
        MaterialFamily::Shaded => shaded_shader(roles),
    }
}

/// `minfilter`/`magfilter` Linear, everything else unset: the baseline every sampler gets
/// unless the role says otherwise.
fn linear() -> SamplerSettings {
    SamplerSettings {
        minfilter: Some(Filter::Linear),
        magfilter: Some(Filter::Linear),
        ..SamplerSettings::default()
    }
}

/// The pre-Fox sampler a role binds to and its default attributes; `None` for Fox-only roles.
pub fn sampler_for_role(role: TextureRole) -> Option<(&'static str, SamplerSettings)> {
    let (name, srgb) = match role {
        TextureRole::Base => ("DiffuseMap", true),
        TextureRole::Normal => ("NormalMap", false),
        TextureRole::Specular => ("SpecularMap", true),
        TextureRole::Environment => {
            return Some((
                "EnvironmentMap",
                SamplerSettings {
                    srgb: Some(false),
                    minfilter: Some(Filter::Anisotropic),
                    maxfilter: None,
                    magfilter: Some(Filter::Linear),
                    mipfilter: None,
                    uaddr: Some(Address::Wrap),
                    vaddr: Some(Address::Wrap),
                    waddr: Some(Address::Wrap),
                    maxaniso: Some(2),
                },
            ));
        }
        TextureRole::DetailMaterial => return Some(("DetailMaterialMap", linear())),
        TextureRole::DetailNormal => return Some(("DetailNormalMap", linear())),
        TextureRole::Metalness | TextureRole::Reflection | TextureRole::ReflectionMask => {
            return None;
        }
    };
    let mut settings = linear();
    settings.srgb = Some(srgb);
    Some((name, settings))
}

/// The parameters `resolve` regenerates for `family` when no `prefox` table is
/// stored — Metal's two; every other family's list is empty.
pub(crate) fn default_parameters(family: MaterialFamily) -> &'static [(&'static str, [f32; 4])] {
    match family {
        MaterialFamily::Metal => &[
            ("Reflection", [1.0, 1.0, 1.0, 0.0]),
            ("Shininess", [0.9, 0.0, 0.0, 1.0]),
        ],
        _ => &[],
    }
}

/// The inverse of [`sampler_for_role`]: `None` for any name that is not a canonical role's
/// pre-Fox sampler (it stays a native sampler).
pub fn role_for_sampler(sampler: &str) -> Option<TextureRole> {
    match sampler {
        "DiffuseMap" => Some(TextureRole::Base),
        "NormalMap" => Some(TextureRole::Normal),
        "SpecularMap" => Some(TextureRole::Specular),
        "EnvironmentMap" => Some(TextureRole::Environment),
        "DetailMaterialMap" => Some(TextureRole::DetailMaterial),
        "DetailNormalMap" => Some(TextureRole::DetailNormal),
        _ => None,
    }
}

/// What a `.model` exporter writes for `material`: the `prefox` table when present, else the
/// family defaults, with the engine-neutral booleans always owning their states.
#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedPreFox {
    /// The shader name.
    pub shader: String,
    /// In `STATE_ORDER`; a native table's extra states (`shadowcaster`) follow in stored
    /// order. `two_sided` always owns `twosided`; `transparent` owns `alphablend` alone over
    /// a native table and the `zwrite`/`alphatest`/`alphablend` triple over the family set.
    pub states: Vec<(String, u32)>,
    /// `(sampler name, settings, texture index)`: canonical roles first in `TextureRole`
    /// order, then `prefox.textures` verbatim with the settings `prefox.samplers` holds for
    /// that name (or all-`None` settings).
    pub samplers: Vec<(String, SamplerSettings, usize)>,
    /// Canonical roles pre-Fox has no sampler for (the caller reports
    /// `material_texture_unused`).
    pub unused_roles: Vec<TextureRole>,
    /// `prefox.textures` names `prefox.samplers` did not define; they export with the
    /// all-`None` settings and the caller reports `sampler_settings_defaulted`.
    pub defaulted_samplers: Vec<String>,
    /// Family defaults (only without a `prefox` table) < `material.parameters` <
    /// `prefox.parameters`; a repeated name keeps its position and takes the later value.
    pub parameters: Vec<(String, Vec<f32>)>,
}

/// Sets `states`' `name` entry in place, or appends it.
fn set_state(states: &mut Vec<(String, u32)>, name: &str, value: u32) {
    match states.iter_mut().find(|(existing, _)| existing == name) {
        Some((_, slot)) => *slot = value,
        None => states.push((name.to_string(), value)),
    }
}

/// `transparent` owns the `zwrite`/`alphatest`/`alphablend` triple — called only for the
/// family-default set; over a native table it owns `alphablend` alone (see `resolve`).
fn apply_transparent(states: &mut Vec<(String, u32)>, transparent: bool) {
    for (name, value) in state_set(transparent) {
        if matches!(name, "zwrite" | "alphatest" | "alphablend") {
            set_state(states, name, value);
        }
    }
}

/// Merges `(name, value)` pairs in order: a name already present keeps its position and
/// takes the new value.
fn merge(into: &mut Vec<(String, Vec<f32>)>, name: &str, value: Vec<f32>) {
    match into.iter_mut().find(|(existing, _)| existing == name) {
        Some((_, slot)) => *slot = value,
        None => into.push((name.to_string(), value)),
    }
}

/// Resolves `material` for a `.model` export.
pub fn resolve(material: &Material) -> ResolvedPreFox {
    let prefox = material.prefox.as_ref();
    let mut roles: Vec<TextureRole> = material.textures.iter().map(|(role, _)| *role).collect();
    roles.sort_unstable();
    roles.dedup();

    let mut states: Vec<(String, u32)> = match prefox {
        Some(prefox) => {
            // The native order re-sorted: the STATE_ORDER states first in that order, the
            // rest in stored order.
            let mut sorted: Vec<(String, u32)> = STATE_ORDER
                .iter()
                .filter_map(|name| {
                    prefox
                        .states
                        .iter()
                        .find(|(stored, _)| stored == name)
                        .map(|(stored, value)| (stored.clone(), *value))
                })
                .collect();
            for (name, value) in &prefox.states {
                if !STATE_ORDER.contains(&name.as_str()) {
                    sorted.push((name.clone(), *value));
                }
            }
            sorted
        }
        None => state_set(material.family == MaterialFamily::Glass)
            .map(|(n, v)| (n.to_string(), v))
            .into(),
    };
    if let Some(two_sided) = material.two_sided {
        set_state(&mut states, "twosided", u32::from(two_sided));
    }
    if let Some(transparent) = material.transparent {
        match prefox {
            // The boolean was read off `alphablend` alone, so over a stored table it owns
            // `alphablend` alone — rewriting the whole triple would clobber a stored
            // `alphatest` the import never looked at.
            Some(_) => set_state(&mut states, "alphablend", u32::from(transparent)),
            None => apply_transparent(&mut states, transparent),
        }
    }

    let mut samplers = Vec::new();
    let mut unused_roles = Vec::new();
    let mut ordered: Vec<(TextureRole, usize)> = material.textures.clone();
    ordered.sort_by_key(|(role, _)| *role);
    for (role, texture) in ordered {
        match sampler_for_role(role) {
            Some((name, defaults)) => {
                let mut defaults = defaults;
                // `Base_Tex_LIN` reads the map linear: the `DiffuseMap` it
                // becomes gets `srgb="0"`.
                if role == TextureRole::Base
                    && material.fox.as_ref().is_some_and(|fox| fox.base_linear)
                {
                    defaults.srgb = Some(false);
                }
                // A stored row replaces the role defaults wholesale: an attribute absent from
                // the `.mtl` resolves to `None`, not to the generated default.
                let settings = prefox
                    .and_then(|prefox| prefox.samplers.iter().find(|(stored, _)| stored == name))
                    .map(|(_, stored)| stored.clone())
                    .unwrap_or(defaults);
                samplers.push((name.to_string(), settings, texture));
            }
            None => unused_roles.push(role),
        }
    }
    let mut defaulted_samplers = Vec::new();
    if let Some(prefox) = prefox {
        for (name, texture) in &prefox.textures {
            let settings = prefox
                .samplers
                .iter()
                .find(|(stored, _)| stored == name)
                .map(|(_, settings)| settings.clone());
            if settings.is_none() {
                defaulted_samplers.push(name.clone());
            }
            samplers.push((name.clone(), settings.unwrap_or_default(), *texture));
        }
    }

    let mut parameters = Vec::new();
    if prefox.is_none() {
        for (name, value) in default_parameters(material.family) {
            merge(&mut parameters, name, value.to_vec());
        }
    }
    for (name, value) in &material.parameters {
        merge(&mut parameters, name, value.to_vec());
    }
    if let Some(prefox) = prefox {
        for (name, value) in &prefox.parameters {
            merge(&mut parameters, name, value.clone());
        }
    }

    ResolvedPreFox {
        shader: prefox.map_or_else(
            || default_shader(material.family, &roles).to_string(),
            |prefox| prefox.shader.clone(),
        ),
        states,
        samplers,
        unused_roles,
        defaulted_samplers,
        parameters,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::materials::{FoxMaterial, PreFoxMaterial};

    /// The state's current value, 0 when absent.
    fn state(states: &[(String, u32)], name: &str) -> u32 {
        states
            .iter()
            .find(|(existing, _)| existing == name)
            .map_or(0, |(_, value)| *value)
    }

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
    fn shaded_ladder() {
        assert_eq!(shaded_shader(&[]), "Basic_C");
        assert_eq!(shaded_shader(&[TextureRole::Normal]), "Basic_CN");
        assert_eq!(
            shaded_shader(&[TextureRole::Normal, TextureRole::Specular]),
            "Basic_CNS"
        );
        assert_eq!(shaded_shader(&[TextureRole::Specular]), "Basic_CNS");
        assert_eq!(
            shaded_shader(&[
                TextureRole::Normal,
                TextureRole::Specular,
                TextureRole::Environment
            ]),
            "Basic_CNSR"
        );
    }

    #[test]
    fn state_sets() {
        assert_eq!(
            state_set(false),
            [
                ("ztest", 1),
                ("zwrite", 1),
                ("twosided", 0),
                ("alphatest", 1),
                ("alpharef", 0),
                ("alphablend", 0),
                ("blendmode", 0)
            ]
        );
        assert_eq!(
            state_set(true),
            [
                ("ztest", 1),
                ("zwrite", 0),
                ("twosided", 0),
                ("alphatest", 0),
                ("alpharef", 0),
                ("alphablend", 1),
                ("blendmode", 0)
            ]
        );
    }

    #[test]
    fn resolve_bare_shaded() {
        let mut mat = material(MaterialFamily::Shaded);
        mat.textures = vec![(TextureRole::Base, 0), (TextureRole::Normal, 1)];
        let resolved = resolve(&mat);
        assert_eq!(resolved.shader, "Basic_CN");
        assert_eq!(
            resolved.states,
            state_set(false)
                .into_iter()
                .map(|(name, value)| (name.to_string(), value))
                .collect::<Vec<_>>()
        );
        assert_eq!(resolved.samplers.len(), 2);
        assert_eq!(resolved.samplers[0].0, "DiffuseMap");
        assert_eq!(resolved.samplers[0].1.srgb, Some(true));
        assert_eq!(resolved.samplers[0].2, 0);
        assert_eq!(resolved.samplers[1].0, "NormalMap");
        assert_eq!(resolved.samplers[1].1.srgb, Some(false));
        assert_eq!(resolved.samplers[1].2, 1);
        assert!(resolved.unused_roles.is_empty());
    }

    #[test]
    fn role_for_sampler_names_the_six_canonical_samplers() {
        for (name, role) in [
            ("DiffuseMap", TextureRole::Base),
            ("NormalMap", TextureRole::Normal),
            ("SpecularMap", TextureRole::Specular),
            ("EnvironmentMap", TextureRole::Environment),
            ("DetailMaterialMap", TextureRole::DetailMaterial),
            ("DetailNormalMap", TextureRole::DetailNormal),
        ] {
            assert_eq!(role_for_sampler(name), Some(role), "{name}");
        }
        assert_eq!(role_for_sampler("DiffuseMap2"), None);
    }

    #[test]
    fn a_metal_material_without_a_table_gets_reflection_parameters() {
        let resolved = resolve(&material(MaterialFamily::Metal));
        assert_eq!(
            resolved.parameters,
            vec![
                ("Reflection".to_string(), vec![1.0, 1.0, 1.0, 0.0]),
                ("Shininess".to_string(), vec![0.9, 0.0, 0.0, 1.0]),
            ]
        );
    }

    #[test]
    fn resolve_glass_is_transparent() {
        let resolved = resolve(&material(MaterialFamily::Glass));
        assert_eq!(resolved.shader, "Basic_C");
        assert_eq!(
            resolved.states,
            state_set(true)
                .into_iter()
                .map(|(name, value)| (name.to_string(), value))
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn two_sided_sets_its_state() {
        let mut mat = material(MaterialFamily::Shaded);
        mat.two_sided = Some(true);
        let resolved = resolve(&mat);
        assert_eq!(state(&resolved.states, "twosided"), 1);
    }

    #[test]
    fn native_table_states_are_resorted() {
        let mut mat = material(MaterialFamily::Shaded);
        mat.prefox = Some(PreFoxMaterial {
            shader: "Basic_C".to_string(),
            states: vec![
                ("blendmode".to_string(), 0),
                ("shadowcaster".to_string(), 1),
                ("alphablend".to_string(), 0),
                ("ztest".to_string(), 1),
                ("alphatest".to_string(), 1),
                ("twosided".to_string(), 0),
                ("zwrite".to_string(), 1),
                ("alpharef".to_string(), 0),
            ],
            samplers: vec![],
            textures: vec![],
            parameters: vec![],
        });
        let resolved = resolve(&mat);
        let names: Vec<&str> = resolved
            .states
            .iter()
            .map(|(name, _)| name.as_str())
            .collect();
        assert_eq!(
            names,
            [
                "ztest",
                "zwrite",
                "twosided",
                "alphatest",
                "alpharef",
                "alphablend",
                "blendmode",
                "shadowcaster"
            ]
        );
        assert_eq!(state(&resolved.states, "shadowcaster"), 1);
    }

    #[test]
    fn transparent_owns_only_alphablend_over_the_table() {
        let mut mat = material(MaterialFamily::Shaded);
        mat.transparent = Some(false);
        mat.prefox = Some(PreFoxMaterial {
            shader: "Basic_C".to_string(),
            states: vec![
                ("ztest".to_string(), 1),
                ("zwrite".to_string(), 0),
                ("alphatest".to_string(), 0),
                ("alphablend".to_string(), 1),
            ],
            samplers: vec![],
            textures: vec![],
            parameters: vec![],
        });
        let resolved = resolve(&mat);
        assert_eq!(state(&resolved.states, "alphablend"), 0);
        // The rest of the stored triple is the table's, not the boolean's.
        assert_eq!(state(&resolved.states, "zwrite"), 0);
        assert_eq!(state(&resolved.states, "alphatest"), 0);
    }

    #[test]
    fn transparent_owns_the_triple_without_a_table() {
        let mut mat = material(MaterialFamily::Shaded);
        mat.transparent = Some(true);
        let resolved = resolve(&mat);
        assert_eq!(state(&resolved.states, "zwrite"), 0);
        assert_eq!(state(&resolved.states, "alphatest"), 0);
        assert_eq!(state(&resolved.states, "alphablend"), 1);
    }

    #[test]
    fn transparent_leaves_a_stored_alphatest_alone() {
        // Card heads in export files store `alphatest: 1, alphablend: 1`; `transparent`
        // was read off `alphablend` alone and must not rewrite `alphatest`.
        let mut mat = material(MaterialFamily::Shaded);
        mat.transparent = Some(true);
        mat.prefox = Some(PreFoxMaterial {
            shader: "Basic_C".to_string(),
            states: vec![("alphatest".to_string(), 1), ("alphablend".to_string(), 1)],
            samplers: vec![],
            textures: vec![],
            parameters: vec![],
        });
        let resolved = resolve(&mat);
        assert_eq!(state(&resolved.states, "alphatest"), 1);
        assert_eq!(state(&resolved.states, "alphablend"), 1);
    }

    #[test]
    fn a_linear_fox_base_stays_linear() {
        // `Base_Tex_LIN`'s `fox.base_linear` writes `srgb="0"` on the
        // `DiffuseMap` it maps to.
        let mut mat = material(MaterialFamily::Shadeless);
        mat.textures = vec![(TextureRole::Base, 0)];
        mat.fox = Some(FoxMaterial {
            shader: "fox3ddf_blin".to_string(),
            technique: "fox3DDF_Blin".to_string(),
            alpha_flags: 0,
            shadow_flags: 0,
            cast_shadow: None,
            invisible: None,
            base_linear: true,
            textures: vec![],
            parameters: vec![],
        });
        let (_, settings, _) = &resolve(&mat).samplers[0];
        assert_eq!(settings.srgb, Some(false));
        // Without the flag the role default `srgb="1"` stands.
        mat.fox.as_mut().expect("fox").base_linear = false;
        let (_, settings, _) = &resolve(&mat).samplers[0];
        assert_eq!(settings.srgb, Some(true));
    }

    #[test]
    fn samplers_come_out_in_role_order() {
        // A source file that lists its normal map before its diffuse map:
        // the written samplers keep the role order, not the listing's.
        let mut mat = material(MaterialFamily::Shaded);
        mat.textures = vec![(TextureRole::Normal, 1), (TextureRole::Base, 0)];
        let samplers = resolve(&mat).samplers;
        let names: Vec<&str> = samplers.iter().map(|(name, _, _)| name.as_str()).collect();
        assert_eq!(names, ["DiffuseMap", "NormalMap"]);
        assert_eq!(samplers[0].2, 0);
        assert_eq!(samplers[1].2, 1);
    }

    #[test]
    fn the_environment_sampler_reads_linear() {
        // The cubemap is not a colour map: `srgb="0"`, and the anisotropic
        // filtering the game's files carry.
        let mut mat = material(MaterialFamily::Shaded);
        mat.textures = vec![(TextureRole::Environment, 0)];
        let (name, settings, texture) = &resolve(&mat).samplers[0];
        assert_eq!(name, "EnvironmentMap");
        assert_eq!(settings.srgb, Some(false));
        assert_eq!(settings.minfilter, Some(Filter::Anisotropic));
        assert_eq!(*texture, 0);
    }

    #[test]
    fn stored_sampler_settings_replace_the_role_defaults() {
        // A `.mtl` sampler with no `srgb` attribute resolves to `srgb: None`, not to the
        // role's generated default, or the `.model → IR → .model` round trip rewrites it.
        let mut mat = material(MaterialFamily::Shaded);
        mat.textures = vec![(TextureRole::Base, 0)];
        let stored = SamplerSettings {
            uaddr: Some(Address::Clamp),
            ..SamplerSettings::default()
        };
        mat.prefox = Some(PreFoxMaterial {
            shader: "Basic_C".to_string(),
            states: vec![],
            samplers: vec![("DiffuseMap".to_string(), stored)],
            textures: vec![],
            parameters: vec![],
        });
        let resolved = resolve(&mat);
        assert_eq!(
            resolved.samplers,
            vec![(
                "DiffuseMap".to_string(),
                SamplerSettings {
                    uaddr: Some(Address::Clamp),
                    ..SamplerSettings::default()
                },
                0
            )]
        );
    }

    #[test]
    fn a_native_table_skips_the_family_defaults() {
        let mut mat = material(MaterialFamily::Metal);
        mat.prefox = Some(PreFoxMaterial {
            shader: "Basic_CNSR".to_string(),
            states: vec![],
            samplers: vec![],
            textures: vec![],
            parameters: vec![("Shininess".to_string(), vec![0.5])],
        });
        assert_eq!(
            resolve(&mat).parameters,
            vec![("Shininess".to_string(), vec![0.5])]
        );
    }

    #[test]
    fn native_textures_follow_the_canonical_samplers() {
        let mut mat = material(MaterialFamily::Shaded);
        mat.textures = vec![(TextureRole::Base, 0)];
        mat.prefox = Some(PreFoxMaterial {
            shader: "Basic_C".to_string(),
            states: vec![],
            samplers: vec![(
                "RoughnessMap".to_string(),
                SamplerSettings {
                    srgb: Some(false),
                    minfilter: Some(Filter::Point),
                    maxfilter: None,
                    magfilter: None,
                    mipfilter: None,
                    uaddr: Some(Address::Clamp),
                    vaddr: None,
                    waddr: None,
                    maxaniso: None,
                },
            )],
            textures: vec![("RoughnessMap".to_string(), 2)],
            parameters: vec![],
        });
        let resolved = resolve(&mat);
        assert_eq!(resolved.samplers.len(), 2);
        assert_eq!(resolved.samplers[0].0, "DiffuseMap");
        assert_eq!(resolved.samplers[1].0, "RoughnessMap");
        assert_eq!(resolved.samplers[1].1.srgb, Some(false));
        assert_eq!(resolved.samplers[1].1.minfilter, Some(Filter::Point));
        assert_eq!(resolved.samplers[1].1.uaddr, Some(Address::Clamp));
        assert_eq!(resolved.samplers[1].2, 2);
    }
}
