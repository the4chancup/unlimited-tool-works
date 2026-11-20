//! The canonical playing-style enum: the 22 values of the PES 20/21 list. The
//! stored value is a per-version index into `schema::playstyle`'s lists; this
//! enum carries no version knowledge.

/// One playing style, in the PES 20/21 list's order (`None` is index 0).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PlayStyle {
    /// No playing style.
    None,
    /// Goal Poacher.
    GoalPoacher,
    /// Dummy Runner.
    DummyRunner,
    /// Fox in the Box.
    FoxInTheBox,
    /// Target Man.
    TargetMan,
    /// Creative Playmaker.
    CreativePlaymaker,
    /// Prolific Winger.
    ProlificWinger,
    /// Roaming Flank.
    RoamingFlank,
    /// Crossing Specialist.
    CrossingSpecialist,
    /// Classic No. 10.
    ClassicNo10,
    /// Hole Player.
    HolePlayer,
    /// Box to Box.
    BoxToBox,
    /// The Destroyer.
    TheDestroyer,
    /// Orchestrator.
    Orchestrator,
    /// Anchor Man.
    AnchorMan,
    /// Offensive Fullback.
    OffensiveFullback,
    /// Fullback Finisher.
    FullbackFinisher,
    /// Defensive Fullback.
    DefensiveFullback,
    /// Build Up.
    BuildUp,
    /// Extra Frontman.
    ExtraFrontman,
    /// Offensive Goalkeeper.
    OffensiveGoalkeeper,
    /// Defensive Goalkeeper.
    DefensiveGoalkeeper,
    /// The Instigator (PES 2014 only).
    TheInstigator,
    /// Solid Player (PES 2014 only).
    SolidPlayer,
}

impl PlayStyle {
    /// Every value, in the canonical order.
    pub const ALL: [PlayStyle; 24] = [
        PlayStyle::None,
        PlayStyle::GoalPoacher,
        PlayStyle::DummyRunner,
        PlayStyle::FoxInTheBox,
        PlayStyle::TargetMan,
        PlayStyle::CreativePlaymaker,
        PlayStyle::ProlificWinger,
        PlayStyle::RoamingFlank,
        PlayStyle::CrossingSpecialist,
        PlayStyle::ClassicNo10,
        PlayStyle::HolePlayer,
        PlayStyle::BoxToBox,
        PlayStyle::TheDestroyer,
        PlayStyle::Orchestrator,
        PlayStyle::AnchorMan,
        PlayStyle::OffensiveFullback,
        PlayStyle::FullbackFinisher,
        PlayStyle::DefensiveFullback,
        PlayStyle::BuildUp,
        PlayStyle::ExtraFrontman,
        PlayStyle::OffensiveGoalkeeper,
        PlayStyle::DefensiveGoalkeeper,
        PlayStyle::TheInstigator,
        PlayStyle::SolidPlayer,
    ];
}
