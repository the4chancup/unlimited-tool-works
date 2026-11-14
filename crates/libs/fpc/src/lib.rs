//! Full Player Customization as data: the kit-config values an FPC team's
//! kits carry, the three player-appearance presets, and the interference
//! rules for settings that break or bend them. Pure data and rules; the
//! kit-config and savefile crates own the formats these values map onto.

mod interference;
mod kit;
mod player;

pub use interference::{Finding, Severity, StripStyle, check};
pub use kit::{KitFpcValues, kit_values};
pub use player::{
    Appearance, HideIds, HideIdsError, Preset, SkinColor, Sleeves, Socks, Tuck,
    custom_skin_available, preset,
};

#[cfg(test)]
mod tests {
    use pes_version::PesVersion;

    use super::*;

    #[test]
    fn kit_values_match_the_text() {
        assert_eq!(
            kit_values(),
            KitFpcValues {
                shirt_model: 176,
                shorts_model: 16,
                collar: 105,
                winter_collar: 105,
            }
        );
    }

    #[test]
    fn custom_skin_only_exists_on_pes_15_to_17() {
        let expected = [true, true, true, false, false, false, false];
        for (version, expected) in PesVersion::ALL.iter().zip(expected) {
            assert_eq!(custom_skin_available(*version), expected, "{version:?}");
        }
    }

    #[test]
    fn presets_match_the_text() {
        assert_eq!(
            preset(Preset::Hide, HideIds::CONVENTION),
            Appearance {
                sleeves: Sleeves::Long,
                tuck: Tuck::Tucked,
                socks: Socks::Short,
                boots_id: 55,
                gloves_id: 11,
                skin_color: SkinColor::Preset,
            }
        );
        assert_eq!(
            preset(Preset::Unhide, HideIds::CONVENTION),
            Appearance {
                sleeves: Sleeves::Short,
                tuck: Tuck::Untucked,
                socks: Socks::Standard,
                boots_id: 0,
                gloves_id: 0,
                skin_color: SkinColor::Preset,
            }
        );
        assert_eq!(
            preset(Preset::PartialHide, HideIds::CONVENTION),
            Appearance {
                skin_color: SkinColor::Custom,
                ..preset(Preset::Unhide, HideIds::CONVENTION)
            }
        );
    }

    // The hide IDs live only inside `HideIds::BOOTS`/`GLOVES`: the
    // constructor's fields are private so an out-of-band pair cannot be
    // built, and the error names the setting and its range.
    #[test]
    fn hide_ids_stay_in_their_bands() {
        let ids = HideIds::new(1, 11).expect("in band");
        assert_eq!((ids.boots(), ids.gloves()), (1, 11));
        let ids = HideIds::new(100, 100).expect("in band");
        assert_eq!((ids.boots(), ids.gloves()), (100, 100));
        assert_eq!(HideIds::new(0, 11), Err(HideIdsError::Boots(0)));
        assert_eq!(HideIds::new(101, 11), Err(HideIdsError::Boots(101)));
        assert_eq!(HideIds::new(55, 10), Err(HideIdsError::Gloves(10)));
        assert_eq!(HideIds::new(55, 101), Err(HideIdsError::Gloves(101)));
        assert_eq!(
            HideIdsError::Boots(627).to_string(),
            "fpc_hide_boots_id = 627: must be 1 to 100 \
             (above 100 the Compiler allocates its own IDs)"
        );
        assert_eq!(
            HideIdsError::Gloves(5).to_string(),
            "fpc_hide_gloves_id = 5: must be 11 to 100 (1 to 10 are goalkeeper gloves)"
        );
    }

    // The hide preset's boots and gloves IDs are the argument's; the other
    // presets' are 0 and 0 whatever the argument.
    #[test]
    fn the_hide_preset_writes_the_given_ids() {
        let ids = HideIds::new(60, 12).expect("in band");
        let hide = preset(Preset::Hide, ids);
        assert_eq!(hide.boots_id, 60);
        assert_eq!(hide.gloves_id, 12);
        for kind in [Preset::Unhide, Preset::PartialHide] {
            let appearance = preset(kind, ids);
            assert_eq!(appearance.boots_id, 0, "{kind:?}");
            assert_eq!(appearance.gloves_id, 0, "{kind:?}");
        }
    }

    #[test]
    fn each_finding_fires_on_exactly_its_condition() {
        let quiet = StripStyle::default();
        assert!(check(&quiet, true).is_empty());
        assert!(check(&quiet, false).is_empty());

        // Inners/undershorts/taping only matter on an FPC player.
        let mut style = StripStyle {
            inners_worn: true,
            undershorts_worn: true,
            wrist_taping: true,
            ..quiet
        };
        assert!(check(&style, false).is_empty());
        let codes: Vec<&str> = check(&style, true).iter().map(|f| f.code).collect();
        assert_eq!(
            codes,
            [
                "fpc_inners_break_hiding",
                "fpc_undershorts_break_hiding",
                "fpc_taping_shows_pieces"
            ]
        );

        // Ankle taping alone also trips the taping rule.
        style = StripStyle {
            ankle_taping: true,
            ..quiet
        };
        assert_eq!(check(&style, true)[0].code, "fpc_taping_shows_pieces");

        // Gloves checkbox + id 0 fires for any player; nonzero id does not.
        style = StripStyle {
            gloves_checkbox: true,
            gloves_id: 0,
            ..quiet
        };
        assert_eq!(
            check(&style, false)[0].code,
            "fpc_gloves_checkbox_winter_gloves"
        );
        style.gloves_id = 3;
        assert!(check(&style, false).is_empty());

        // Custom skin warns only on non-FPC players.
        style = StripStyle {
            skin_color: SkinColor::Custom,
            ..quiet
        };
        assert!(check(&style, true).is_empty());
        assert_eq!(check(&style, false)[0].code, "fpc_custom_skin_on_non_fpc");
    }

    #[test]
    fn severity_orders_info_before_warning_before_error() {
        assert!(Severity::Info < Severity::Warning);
        assert!(Severity::Warning < Severity::Error);
        assert_eq!(
            check(
                &StripStyle {
                    inners_worn: true,
                    gloves_checkbox: true,
                    gloves_id: 0,
                    ..StripStyle::default()
                },
                true
            )
            .iter()
            .map(|f| f.severity)
            .collect::<Vec<_>>(),
            [Severity::Error, Severity::Warning]
        );
    }
}
