//! Entity system — sparse ID-indexed entities.

use gruid_core::Point;

/// Number of spirit inventory slots.
pub const N_SPIRITS: usize = 3;
/// Number of comestible inventory slots.
pub const N_COMESTIBLES: usize = 5;
/// Total inventory size.
pub const INVENTORY_SIZE: usize = N_SPIRITS + N_COMESTIBLES;

/// Type alias for entity IDs (index into entity vec).
pub type Id = usize;

/// First map entity ID (player is always this ID).
pub const FIRST_MAP_ID: Id = INVENTORY_SIZE;
/// Player entity ID.
pub const PLAYER_ID: Id = FIRST_MAP_ID;

/// Invalid position sentinel.
pub const INVALID_POS: Point = Point { x: -1, y: -1 };

/// Render order for drawing precedence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RenderOrder {
    None = 0,
    Item = 1,
    Actor = 2,
}

/// An entity's polymorphic role.
#[derive(Debug, Clone)]
pub enum Role {
    Actor(Actor),
    // Future: Spirit, Comestible, Menhir, Portal, etc.
}

/// An entity in the game world.
#[derive(Debug, Clone)]
pub struct Entity {
    pub name: String,
    pub ch: char,
    pub pos: Point,
    pub known_pos: Point,
    pub seen: bool,
    pub role: Role,
}

impl Entity {
    /// Whether this entity is an alive actor on the map.
    pub fn is_alive(&self) -> bool {
        match &self.role {
            Role::Actor(a) => a.hp > 0 && self.pos != INVALID_POS,
        }
    }

    /// Get actor ref, if the role is Actor.
    pub fn actor(&self) -> Option<&Actor> {
        match &self.role {
            Role::Actor(a) => Some(a),
        }
    }

    /// Get actor mut ref.
    pub fn actor_mut(&mut self) -> Option<&mut Actor> {
        match &mut self.role {
            Role::Actor(a) => Some(a),
        }
    }

    /// Render order for drawing.
    pub fn render_order(&self) -> RenderOrder {
        match &self.role {
            Role::Actor(a) if a.hp > 0 => RenderOrder::Actor,
            _ => RenderOrder::None,
        }
    }
}

/// Behavior state for monster AI.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mindstate {
    Wandering,
    Hunting,
}

/// Monster AI behavior.
#[derive(Debug, Clone)]
pub struct Behavior {
    pub path: Vec<Point>,
    pub target: Point,
    pub guard: Point,
    pub state: Mindstate,
    pub kind: MonsterKind,
}

/// Monster kinds (subset for initial port).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MonsterKind {
    HungryRat,
    BerserkingSpider,
    ConfusingEye,
    ThunderPorcupine,
    AcidMound,
    BarkingHound,
    VenomousViper,
    FireLlama,
    LashingFrog,
    RampagingBoar,
    ExplodingNadre,
    TemporalCat,
    WindFox,
    BlinkButterfly,
}

/// Monster data template.
pub struct MonsterData {
    pub kind: MonsterKind,
    pub name: &'static str,
    pub ch: char,
    pub attack: i32,
    pub defense: i32,
    pub hp: i32,
}

/// Static table of monster definitions.
pub const MONSTER_DATA: &[MonsterData] = &[
    MonsterData {
        kind: MonsterKind::HungryRat,
        name: "hungry rat",
        ch: 'r',
        attack: 2,
        defense: 0,
        hp: 3,
    },
    MonsterData {
        kind: MonsterKind::BerserkingSpider,
        name: "berserking spider",
        ch: 's',
        attack: 2,
        defense: 1,
        hp: 3,
    },
    MonsterData {
        kind: MonsterKind::ConfusingEye,
        name: "confusing eye",
        ch: 'e',
        attack: 2,
        defense: 0,
        hp: 2,
    },
    MonsterData {
        kind: MonsterKind::ThunderPorcupine,
        name: "thunder porcupine",
        ch: 'p',
        attack: 2,
        defense: 0,
        hp: 3,
    },
    MonsterData {
        kind: MonsterKind::AcidMound,
        name: "acid mound",
        ch: 'a',
        attack: 2,
        defense: 0,
        hp: 5,
    },
    MonsterData {
        kind: MonsterKind::BarkingHound,
        name: "barking hound",
        ch: 'h',
        attack: 3,
        defense: 0,
        hp: 4,
    },
    MonsterData {
        kind: MonsterKind::VenomousViper,
        name: "venomous viper",
        ch: 'v',
        attack: 2,
        defense: 1,
        hp: 4,
    },
    MonsterData {
        kind: MonsterKind::FireLlama,
        name: "fire llama",
        ch: 'l',
        attack: 2,
        defense: 0,
        hp: 4,
    },
    MonsterData {
        kind: MonsterKind::LashingFrog,
        name: "lashing frog",
        ch: 'F',
        attack: 2,
        defense: 1,
        hp: 4,
    },
    MonsterData {
        kind: MonsterKind::RampagingBoar,
        name: "rampaging boar",
        ch: 'B',
        attack: 3,
        defense: 0,
        hp: 4,
    },
    MonsterData {
        kind: MonsterKind::ExplodingNadre,
        name: "exploding nadre",
        ch: 'n',
        attack: 2,
        defense: 3,
        hp: 1,
    },
    MonsterData {
        kind: MonsterKind::TemporalCat,
        name: "temporal cat",
        ch: 'c',
        attack: 2,
        defense: 0,
        hp: 4,
    },
    MonsterData {
        kind: MonsterKind::WindFox,
        name: "wind fox",
        ch: 'f',
        attack: 2,
        defense: 0,
        hp: 4,
    },
    MonsterData {
        kind: MonsterKind::BlinkButterfly,
        name: "blinking butterfly",
        ch: 'b',
        attack: 2,
        defense: 3,
        hp: 2,
    },
];

/// Actor component (HP, combat stats).
#[derive(Debug, Clone)]
pub struct Actor {
    pub hp: i32,
    pub max_hp: i32,
    pub attack: i32,
    pub defense: i32,
    pub behavior: Option<Behavior>,
}

impl Actor {
    pub fn new(attack: i32, defense: i32, max_hp: i32) -> Self {
        Self {
            hp: max_hp,
            max_hp,
            attack,
            defense,
            behavior: None,
        }
    }

    pub fn new_monster(data: &MonsterData, kind: MonsterKind) -> Self {
        Self {
            hp: data.hp,
            max_hp: data.hp,
            attack: data.attack,
            defense: data.defense,
            behavior: Some(Behavior {
                path: Vec::new(),
                target: INVALID_POS,
                guard: INVALID_POS,
                state: Mindstate::Wandering,
                kind,
            }),
        }
    }
}

/// Spawn pool tiers.
pub const EARLY_MONSTERS: &[MonsterKind] = &[
    MonsterKind::BerserkingSpider,
    MonsterKind::ConfusingEye,
    MonsterKind::HungryRat,
    MonsterKind::ThunderPorcupine,
];

pub const MID_MONSTERS: &[MonsterKind] = &[
    MonsterKind::AcidMound,
    MonsterKind::BarkingHound,
    MonsterKind::BlinkButterfly,
    MonsterKind::ExplodingNadre,
    MonsterKind::FireLlama,
    MonsterKind::LashingFrog,
    MonsterKind::RampagingBoar,
    MonsterKind::TemporalCat,
    MonsterKind::VenomousViper,
    MonsterKind::WindFox,
];

/// Get MonsterData for a given kind.
pub fn monster_data(kind: MonsterKind) -> &'static MonsterData {
    MONSTER_DATA
        .iter()
        .find(|d| d.kind == kind)
        .expect("unknown monster kind")
}
