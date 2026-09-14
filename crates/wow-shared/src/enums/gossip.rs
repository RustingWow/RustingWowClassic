use serde::{Deserialize, Serialize};

use super::db_enum;

/// CMaNGOS `gossip_menu_option.option_id` — what the gossip row does.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum GossipOptionKind {
    #[default]
    #[serde(rename = "NONE")]
    None,
    #[serde(rename = "GOSSIP")]
    Gossip,
    #[serde(rename = "QUESTGIVER")]
    QuestGiver,
    #[serde(rename = "VENDOR")]
    Vendor,
    #[serde(rename = "TAXIVENDOR")]
    TaxiVendor,
    #[serde(rename = "TRAINER")]
    Trainer,
    #[serde(rename = "SPIRITHEALER")]
    SpiritHealer,
    #[serde(rename = "SPIRITGUIDE")]
    SpiritGuide,
    #[serde(rename = "INNKEEPER")]
    Innkeeper,
    #[serde(rename = "BANKER")]
    Banker,
    #[serde(rename = "PETITIONER")]
    Petitioner,
    #[serde(rename = "TABARDDESIGNER")]
    TabardDesigner,
    #[serde(rename = "BATTLEFIELD")]
    Battlefield,
    #[serde(rename = "AUCTIONEER")]
    Auctioneer,
    #[serde(rename = "STABLEPET")]
    StablePet,
    #[serde(rename = "ARMORER")]
    Armorer,
    #[serde(rename = "UNLEARN_TALENTS")]
    UnlearnTalents,
    #[serde(rename = "UNLEARN_PET_SKILLS")]
    UnlearnPetSkills,
    /// Playerbots row present in classic-db (`option_id` 99).
    #[serde(rename = "BOT")]
    Bot,
}

db_enum!(
    GossipOptionKind,
    u8,
    None => 0, "NONE",
    Gossip => 1, "GOSSIP",
    QuestGiver => 2, "QUESTGIVER",
    Vendor => 3, "VENDOR",
    TaxiVendor => 4, "TAXIVENDOR",
    Trainer => 5, "TRAINER",
    SpiritHealer => 6, "SPIRITHEALER",
    SpiritGuide => 7, "SPIRITGUIDE",
    Innkeeper => 8, "INNKEEPER",
    Banker => 9, "BANKER",
    Petitioner => 10, "PETITIONER",
    TabardDesigner => 11, "TABARDDESIGNER",
    Battlefield => 12, "BATTLEFIELD",
    Auctioneer => 13, "AUCTIONEER",
    StablePet => 14, "STABLEPET",
    Armorer => 15, "ARMORER",
    UnlearnTalents => 16, "UNLEARN_TALENTS",
    UnlearnPetSkills => 17, "UNLEARN_PET_SKILLS",
    Bot => 99, "BOT",
);

impl GossipOptionKind {
    /// Kinds the world server actually maps to a gossip action.
    pub fn is_handled(self) -> bool {
        matches!(
            self,
            Self::None | Self::Gossip | Self::Vendor | Self::QuestGiver
        )
    }
}

/// CMaNGOS / Trinity `GossipOptionIcon` — bubble shown next to the gossip row.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum GossipOptionIcon {
    #[default]
    #[serde(rename = "CHAT")]
    Chat,
    #[serde(rename = "VENDOR")]
    Vendor,
    #[serde(rename = "TAXI")]
    Taxi,
    #[serde(rename = "TRAINER")]
    Trainer,
    #[serde(rename = "INTERACT_1")]
    Interact1,
    #[serde(rename = "INTERACT_2")]
    Interact2,
    #[serde(rename = "MONEY_BAG")]
    MoneyBag,
    #[serde(rename = "TALK")]
    Talk,
    #[serde(rename = "TABARD")]
    Tabard,
    #[serde(rename = "BATTLE")]
    Battle,
    #[serde(rename = "DOT")]
    Dot,
    #[serde(rename = "CHAT_11")]
    Chat11,
    #[serde(rename = "CHAT_12")]
    Chat12,
}

db_enum!(
    GossipOptionIcon,
    u8,
    Chat => 0, "CHAT",
    Vendor => 1, "VENDOR",
    Taxi => 2, "TAXI",
    Trainer => 3, "TRAINER",
    Interact1 => 4, "INTERACT_1",
    Interact2 => 5, "INTERACT_2",
    MoneyBag => 6, "MONEY_BAG",
    Talk => 7, "TALK",
    Tabard => 8, "TABARD",
    Battle => 9, "BATTLE",
    Dot => 10, "DOT",
    Chat11 => 11, "CHAT_11",
    Chat12 => 12, "CHAT_12",
);
