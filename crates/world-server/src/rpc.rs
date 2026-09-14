use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

use serde::{Deserialize, Serialize};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{Mutex, mpsc, oneshot};
use wow_shared::Position;
use wow_world_messages::vanilla::CreatureFamily;

use crate::catalog::{ItemRow, NpcTextPage, QuestRow};
use crate::creature::{Creature, Gossip, GossipMenu};
use crate::gameobject::GameObject;
use crate::player::Player;
use crate::quest::{GossipQuestItem, KillCredit, QuestStateChange};
use crate::world::{
    Attack, Chat, LootOffer, MeleeHit, Movement, PlayerMailbox, SpokenChat, VendorOffer, World,
    WorldEvent,
};

#[derive(Debug, Serialize, Deserialize)]
struct Frame {
    id: u64,
    body: FrameBody,
}

#[derive(Debug, Serialize, Deserialize)]
enum FrameBody {
    Join {
        map_id: u32,
        player: Player,
    },
    JoinOk {
        others: Vec<Player>,
        creatures: Vec<WireCreature>,
        gameobjects: Vec<GameObject>,
    },
    Leave {
        guid: u64,
    },
    Move {
        guid: u64,
        position: Position,
    },
    Speak {
        chat: Chat,
    },
    Attack {
        attacker: u64,
        target: u64,
    },
    StopAttack {
        attacker: u64,
    },
    StandState {
        guid: u64,
        state: u8,
    },
    GossipHello {
        player: u64,
        npc: u64,
    },
    GossipSelect {
        player: u64,
        npc: u64,
        option: u32,
    },
    ListVendor {
        player: u64,
        npc: u64,
    },
    BuyItem {
        player: u64,
        npc: u64,
        item: u32,
        amount: u32,
    },
    OpenLoot {
        player: u64,
        npc: u64,
    },
    TakeLoot {
        player: u64,
        index: u8,
    },
    CloseLoot {
        player: u64,
    },
    QueryItem {
        entry: u32,
    },
    Item {
        item: Option<ItemRow>,
    },
    QueryPlayer {
        guid: u64,
    },
    Player {
        player: Option<Player>,
    },
    QueryCreature {
        guid: u64,
    },
    QueryCreatureEntry {
        entry: u32,
    },
    Creature {
        creature: Option<WireCreature>,
    },
    QueryGameObject {
        guid: u64,
    },
    QueryGameObjectEntry {
        entry: u32,
    },
    GameObject {
        object: Option<GameObject>,
    },
    QueryNpcText {
        text_id: u32,
    },
    NpcText {
        pages: [NpcTextPage; 8],
    },
    QueryQuest {
        entry: u32,
    },
    Quest {
        quest: Option<QuestRow>,
    },
    QuestGiverStatus {
        player: u64,
        npc: u64,
    },
    QuestGiverHello {
        player: u64,
        npc: u64,
    },
    QuestGiverQuery {
        player: u64,
        npc: u64,
        quest_id: u32,
    },
    QuestGiverAccept {
        player: u64,
        npc: u64,
        quest_id: u32,
    },
    QuestGiverComplete {
        player: u64,
        npc: u64,
        quest_id: u32,
    },
    QuestGiverChooseReward {
        player: u64,
        npc: u64,
        quest_id: u32,
        reward: u32,
    },
    QuestLogRemove {
        player: u64,
        slot: u8,
    },
    Ack,
    Event(WireEvent),
    Error(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
enum WireEvent {
    PlayerAppeared(Player),
    PlayerLeft {
        guid: u64,
    },
    PlayerMoved {
        guid: u64,
        position: Position,
    },
    Chat(SpokenChat),
    ChatPlayerNotFound {
        name: String,
    },
    CreatureAppeared(WireCreature),
    CreatureLeft {
        guid: u64,
    },
    GameObjectAppeared(GameObject),
    GameObjectLeft {
        guid: u64,
    },
    AttackStarted(Attack),
    AttackStopped(Attack),
    MeleeHit(MeleeHit),
    StandStateAck {
        state: u8,
    },
    PlayerStandState {
        guid: u64,
        state: u8,
    },
    GossipOpened {
        npc: u64,
        menu: GossipMenu,
        quests: Vec<GossipQuestItem>,
        pages: [NpcTextPage; 8],
    },
    GossipClosed,
    QuestGiverStatus {
        npc: u64,
        status: u32,
    },
    QuestList {
        npc: u64,
        title: String,
        quests: Vec<GossipQuestItem>,
    },
    QuestDetails {
        npc: u64,
        quest: QuestRow,
    },
    QuestLogFull,
    QuestLogUpdate {
        player: Player,
    },
    QuestOfferReward {
        npc: u64,
        quest: QuestRow,
    },
    QuestTurnedIn {
        quest_id: u32,
        copper: u32,
        items: Vec<(u32, u32)>,
    },
    QuestKillCredit {
        victim: u64,
        credit: KillCredit,
    },
    QuestObjectivesDone {
        quest_id: u32,
    },
    QuestStateChanged {
        guid: u64,
        change: QuestStateChange,
    },
    VendorOpened {
        npc: u64,
        items: Vec<VendorOffer>,
    },
    LootOpened {
        guid: u64,
        gold: u32,
        items: Vec<LootOffer>,
    },
    LootFailed {
        guid: u64,
        error: u8,
    },
    LootTaken {
        index: u8,
    },
    LootClosed {
        guid: u64,
    },
    MoneyChanged {
        guid: u64,
        copper: u32,
    },
    Notification {
        text: String,
    },
    ForcedTeleport {
        map_id: u32,
        position: Position,
    },
    CreatureMoved {
        guid: u64,
        from: Position,
        to: Position,
        duration_ms: u32,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct WireCreature {
    guid: u64,
    entry: u32,
    name: String,
    sub_name: String,
    display_id: i32,
    faction: i32,
    position: Position,
    health: i32,
    max_health: i32,
    level: i32,
    npc_flags: i32,
    creature_type: u32,
    family: u32,
    civilian: bool,
    hostile: bool,
    dead: bool,
    #[serde(default)]
    lootable: bool,
    gossip: Option<Gossip>,
    loot_id: i32,
    respawn_secs: u32,
    melee_damage: i32,
}

impl From<&Creature> for WireCreature {
    fn from(creature: &Creature) -> Self {
        Self {
            guid: creature.guid,
            entry: creature.entry,
            name: creature.name.clone(),
            sub_name: creature.sub_name.clone(),
            display_id: creature.display_id,
            faction: creature.faction,
            position: creature.position,
            health: creature.health,
            max_health: creature.max_health,
            level: creature.level,
            npc_flags: creature.npc_flags,
            creature_type: creature.creature_type,
            family: family_to_u32(creature.family),
            civilian: creature.civilian,
            hostile: creature.hostile,
            dead: creature.dead,
            lootable: creature.lootable,
            gossip: creature.gossip.clone(),
            loot_id: creature.loot_id,
            respawn_secs: creature.respawn_secs,
            melee_damage: creature.melee_damage,
        }
    }
}

impl From<WireCreature> for Creature {
    fn from(wire: WireCreature) -> Self {
        Self {
            guid: wire.guid,
            entry: wire.entry,
            name: wire.name,
            sub_name: wire.sub_name,
            display_id: wire.display_id,
            faction: wire.faction,
            position: wire.position,
            health: wire.health,
            max_health: wire.max_health,
            level: wire.level,
            npc_flags: wire.npc_flags,
            creature_type: wire.creature_type,
            family: family_from_u32(wire.family),
            civilian: wire.civilian,
            hostile: wire.hostile,
            dead: wire.dead,
            lootable: wire.lootable,
            gossip: wire.gossip,
            loot_id: wire.loot_id,
            respawn_secs: wire.respawn_secs,
            melee_damage: wire.melee_damage,
        }
    }
}

fn family_to_u32(family: CreatureFamily) -> u32 {
    match family {
        CreatureFamily::Wolf => 1,
        _ => 0,
    }
}

fn family_from_u32(value: u32) -> CreatureFamily {
    match value {
        1 => CreatureFamily::Wolf,
        _ => CreatureFamily::None,
    }
}

fn event_to_wire(event: WorldEvent) -> WireEvent {
    match event {
        WorldEvent::PlayerAppeared(player) => WireEvent::PlayerAppeared(player),
        WorldEvent::PlayerLeft { guid } => WireEvent::PlayerLeft { guid },
        WorldEvent::PlayerMoved(movement) => WireEvent::PlayerMoved {
            guid: movement.guid,
            position: movement.position,
        },
        WorldEvent::Chat(chat) => WireEvent::Chat(chat),
        WorldEvent::ChatPlayerNotFound { name } => WireEvent::ChatPlayerNotFound { name },
        WorldEvent::CreatureAppeared(creature) => {
            WireEvent::CreatureAppeared(WireCreature::from(&creature))
        }
        WorldEvent::CreatureLeft { guid } => WireEvent::CreatureLeft { guid },
        WorldEvent::GameObjectAppeared(object) => WireEvent::GameObjectAppeared(object),
        WorldEvent::GameObjectLeft { guid } => WireEvent::GameObjectLeft { guid },
        WorldEvent::AttackStarted(attack) => WireEvent::AttackStarted(attack),
        WorldEvent::AttackStopped(attack) => WireEvent::AttackStopped(attack),
        WorldEvent::MeleeHit(hit) => WireEvent::MeleeHit(hit),
        WorldEvent::StandStateAck { state } => WireEvent::StandStateAck { state },
        WorldEvent::PlayerStandState { guid, state } => WireEvent::PlayerStandState { guid, state },
        WorldEvent::GossipOpened {
            npc,
            menu,
            quests,
            pages,
        } => WireEvent::GossipOpened {
            npc,
            menu,
            quests,
            pages,
        },
        WorldEvent::GossipClosed => WireEvent::GossipClosed,
        WorldEvent::QuestGiverStatus { npc, status } => WireEvent::QuestGiverStatus { npc, status },
        WorldEvent::QuestList { npc, title, quests } => WireEvent::QuestList { npc, title, quests },
        WorldEvent::QuestDetails { npc, quest } => WireEvent::QuestDetails { npc, quest },
        WorldEvent::QuestLogFull => WireEvent::QuestLogFull,
        WorldEvent::QuestLogUpdate { player } => WireEvent::QuestLogUpdate { player },
        WorldEvent::QuestOfferReward { npc, quest } => WireEvent::QuestOfferReward { npc, quest },
        WorldEvent::QuestTurnedIn {
            quest_id,
            copper,
            items,
        } => WireEvent::QuestTurnedIn {
            quest_id,
            copper,
            items,
        },
        WorldEvent::QuestKillCredit { victim, credit } => {
            WireEvent::QuestKillCredit { victim, credit }
        }
        WorldEvent::QuestObjectivesDone { quest_id } => WireEvent::QuestObjectivesDone { quest_id },
        WorldEvent::QuestStateChanged { guid, change } => {
            WireEvent::QuestStateChanged { guid, change }
        }
        WorldEvent::VendorOpened { npc, items } => WireEvent::VendorOpened { npc, items },
        WorldEvent::LootOpened { guid, gold, items } => WireEvent::LootOpened { guid, gold, items },
        WorldEvent::LootFailed { guid, error } => WireEvent::LootFailed { guid, error },
        WorldEvent::LootTaken { index } => WireEvent::LootTaken { index },
        WorldEvent::LootClosed { guid } => WireEvent::LootClosed { guid },
        WorldEvent::MoneyChanged { guid, copper } => WireEvent::MoneyChanged { guid, copper },
        WorldEvent::Notification { text } => WireEvent::Notification { text },
        WorldEvent::ForcedTeleport { map_id, position } => {
            WireEvent::ForcedTeleport { map_id, position }
        }
        WorldEvent::CreatureMoved {
            guid,
            from,
            to,
            duration_ms,
        } => WireEvent::CreatureMoved {
            guid,
            from,
            to,
            duration_ms,
        },
    }
}

fn event_from_wire(event: WireEvent) -> WorldEvent {
    match event {
        WireEvent::PlayerAppeared(player) => WorldEvent::PlayerAppeared(player),
        WireEvent::PlayerLeft { guid } => WorldEvent::PlayerLeft { guid },
        WireEvent::PlayerMoved { guid, position } => {
            WorldEvent::PlayerMoved(Movement::without_packet(guid, position))
        }
        WireEvent::Chat(chat) => WorldEvent::Chat(chat),
        WireEvent::ChatPlayerNotFound { name } => WorldEvent::ChatPlayerNotFound { name },
        WireEvent::CreatureAppeared(creature) => WorldEvent::CreatureAppeared(creature.into()),
        WireEvent::CreatureLeft { guid } => WorldEvent::CreatureLeft { guid },
        WireEvent::GameObjectAppeared(object) => WorldEvent::GameObjectAppeared(object),
        WireEvent::GameObjectLeft { guid } => WorldEvent::GameObjectLeft { guid },
        WireEvent::AttackStarted(attack) => WorldEvent::AttackStarted(attack),
        WireEvent::AttackStopped(attack) => WorldEvent::AttackStopped(attack),
        WireEvent::MeleeHit(hit) => WorldEvent::MeleeHit(hit),
        WireEvent::StandStateAck { state } => WorldEvent::StandStateAck { state },
        WireEvent::PlayerStandState { guid, state } => WorldEvent::PlayerStandState { guid, state },
        WireEvent::GossipOpened {
            npc,
            menu,
            quests,
            pages,
        } => WorldEvent::GossipOpened {
            npc,
            menu,
            quests,
            pages,
        },
        WireEvent::GossipClosed => WorldEvent::GossipClosed,
        WireEvent::QuestGiverStatus { npc, status } => WorldEvent::QuestGiverStatus { npc, status },
        WireEvent::QuestList { npc, title, quests } => WorldEvent::QuestList { npc, title, quests },
        WireEvent::QuestDetails { npc, quest } => WorldEvent::QuestDetails { npc, quest },
        WireEvent::QuestLogFull => WorldEvent::QuestLogFull,
        WireEvent::QuestLogUpdate { player } => WorldEvent::QuestLogUpdate { player },
        WireEvent::QuestOfferReward { npc, quest } => WorldEvent::QuestOfferReward { npc, quest },
        WireEvent::QuestTurnedIn {
            quest_id,
            copper,
            items,
        } => WorldEvent::QuestTurnedIn {
            quest_id,
            copper,
            items,
        },
        WireEvent::QuestKillCredit { victim, credit } => {
            WorldEvent::QuestKillCredit { victim, credit }
        }
        WireEvent::QuestObjectivesDone { quest_id } => WorldEvent::QuestObjectivesDone { quest_id },
        WireEvent::QuestStateChanged { guid, change } => {
            WorldEvent::QuestStateChanged { guid, change }
        }
        WireEvent::VendorOpened { npc, items } => WorldEvent::VendorOpened { npc, items },
        WireEvent::LootOpened { guid, gold, items } => WorldEvent::LootOpened { guid, gold, items },
        WireEvent::LootFailed { guid, error } => WorldEvent::LootFailed { guid, error },
        WireEvent::LootTaken { index } => WorldEvent::LootTaken { index },
        WireEvent::LootClosed { guid } => WorldEvent::LootClosed { guid },
        WireEvent::MoneyChanged { guid, copper } => WorldEvent::MoneyChanged { guid, copper },
        WireEvent::Notification { text } => WorldEvent::Notification { text },
        WireEvent::ForcedTeleport { map_id, position } => {
            WorldEvent::ForcedTeleport { map_id, position }
        }
        WireEvent::CreatureMoved {
            guid,
            from,
            to,
            duration_ms,
        } => WorldEvent::CreatureMoved {
            guid,
            from,
            to,
            duration_ms,
        },
    }
}

async fn write_frame<W: AsyncWriteExt + Unpin>(
    writer: &mut W,
    frame: &Frame,
) -> std::io::Result<()> {
    let bytes = serde_json::to_vec(frame).map_err(std::io::Error::other)?;
    writer.write_u32(bytes.len() as u32).await?;
    writer.write_all(&bytes).await?;
    Ok(())
}

async fn read_frame<R: AsyncReadExt + Unpin>(reader: &mut R) -> std::io::Result<Frame> {
    let len = reader.read_u32().await?;
    if len > 1_000_000 {
        return Err(std::io::Error::other("rpc frame too large"));
    }
    let mut buf = vec![0_u8; len as usize];
    reader.read_exact(&mut buf).await?;
    serde_json::from_slice(&buf).map_err(std::io::Error::other)
}

pub struct MapSession {
    map_id: u32,
    tx: mpsc::UnboundedSender<(FrameBody, Option<oneshot::Sender<FrameBody>>)>,
}

impl MapSession {
    pub fn map_id(&self) -> u32 {
        self.map_id
    }

    pub async fn connect(
        addr: SocketAddr,
        map_id: u32,
        mailbox: PlayerMailbox,
    ) -> anyhow::Result<Self> {
        let stream = TcpStream::connect(addr).await?;
        let (mut reader, mut writer) = stream.into_split();
        let (tx, mut rx) =
            mpsc::unbounded_channel::<(FrameBody, Option<oneshot::Sender<FrameBody>>)>();
        let pending: Arc<Mutex<HashMap<u64, oneshot::Sender<FrameBody>>>> =
            Arc::new(Mutex::new(HashMap::new()));
        let next_id = Arc::new(AtomicU64::new(1));

        let pending_w = pending.clone();
        let ids = next_id.clone();
        tokio::spawn(async move {
            while let Some((body, reply)) = rx.recv().await {
                let id = ids.fetch_add(1, Ordering::Relaxed);
                if let Some(reply) = reply {
                    pending_w.lock().await.insert(id, reply);
                }
                if write_frame(&mut writer, &Frame { id, body }).await.is_err() {
                    break;
                }
            }
        });

        tokio::spawn(async move {
            loop {
                match read_frame(&mut reader).await {
                    Ok(frame) => {
                        if let FrameBody::Event(event) = frame.body {
                            mailbox.send(event_from_wire(event));
                            continue;
                        }
                        if let Some(reply) = pending.lock().await.remove(&frame.id) {
                            let _ = reply.send(frame.body);
                        }
                    }
                    Err(_) => break,
                }
            }
        });

        Ok(Self { map_id, tx })
    }

    pub fn leave(&self, guid: u64) {
        let _ = self.tx.send((FrameBody::Leave { guid }, None));
    }

    async fn request(&self, body: FrameBody) -> anyhow::Result<FrameBody> {
        let (reply_tx, reply_rx) = oneshot::channel();
        self.tx
            .send((body, Some(reply_tx)))
            .map_err(|_| anyhow::anyhow!("map session closed"))?;
        reply_rx
            .await
            .map_err(|_| anyhow::anyhow!("map session closed"))
    }

    pub async fn join(
        &self,
        player: Player,
    ) -> anyhow::Result<(Vec<Player>, Vec<Creature>, Vec<GameObject>)> {
        match self
            .request(FrameBody::Join {
                map_id: self.map_id,
                player,
            })
            .await?
        {
            FrameBody::JoinOk {
                others,
                creatures,
                gameobjects,
            } => Ok((
                others,
                creatures.into_iter().map(Into::into).collect(),
                gameobjects,
            )),
            FrameBody::Error(error) => anyhow::bail!(error),
            _ => anyhow::bail!("unexpected join response"),
        }
    }

    pub async fn broadcast_move(&self, guid: u64, position: Position) -> anyhow::Result<()> {
        self.request(FrameBody::Move { guid, position }).await?;
        Ok(())
    }

    pub async fn speak(&self, chat: Chat) -> anyhow::Result<()> {
        self.request(FrameBody::Speak { chat }).await?;
        Ok(())
    }

    pub async fn start_attack(&self, attacker: u64, target: u64) -> anyhow::Result<()> {
        self.request(FrameBody::Attack { attacker, target }).await?;
        Ok(())
    }

    pub async fn stop_attack(&self, attacker: u64) -> anyhow::Result<()> {
        self.request(FrameBody::StopAttack { attacker }).await?;
        Ok(())
    }

    pub async fn change_stand_state(&self, guid: u64, state: u8) -> anyhow::Result<()> {
        self.request(FrameBody::StandState { guid, state }).await?;
        Ok(())
    }

    pub async fn open_gossip(&self, player: u64, npc: u64) -> anyhow::Result<()> {
        self.request(FrameBody::GossipHello { player, npc }).await?;
        Ok(())
    }

    pub async fn select_gossip_option(
        &self,
        player: u64,
        npc: u64,
        option: u32,
    ) -> anyhow::Result<()> {
        self.request(FrameBody::GossipSelect {
            player,
            npc,
            option,
        })
        .await?;
        Ok(())
    }

    pub async fn list_vendor(&self, player: u64, npc: u64) -> anyhow::Result<()> {
        self.request(FrameBody::ListVendor { player, npc }).await?;
        Ok(())
    }

    pub async fn buy_item(
        &self,
        player: u64,
        npc: u64,
        item: u32,
        amount: u32,
    ) -> anyhow::Result<()> {
        self.request(FrameBody::BuyItem {
            player,
            npc,
            item,
            amount,
        })
        .await?;
        Ok(())
    }

    pub async fn open_loot(&self, player: u64, npc: u64) -> anyhow::Result<()> {
        self.request(FrameBody::OpenLoot { player, npc }).await?;
        Ok(())
    }

    pub async fn take_loot(&self, player: u64, index: u8) -> anyhow::Result<()> {
        self.request(FrameBody::TakeLoot { player, index }).await?;
        Ok(())
    }

    pub async fn close_loot(&self, player: u64) -> anyhow::Result<()> {
        self.request(FrameBody::CloseLoot { player }).await?;
        Ok(())
    }

    pub async fn item(&self, entry: u32) -> anyhow::Result<Option<ItemRow>> {
        match self.request(FrameBody::QueryItem { entry }).await? {
            FrameBody::Item { item } => Ok(item),
            other => anyhow::bail!("unexpected item response: {other:?}"),
        }
    }

    pub async fn player(&self, guid: u64) -> anyhow::Result<Option<Player>> {
        match self.request(FrameBody::QueryPlayer { guid }).await? {
            FrameBody::Player { player } => Ok(player),
            other => anyhow::bail!("unexpected player response: {other:?}"),
        }
    }

    pub async fn creature(&self, guid: u64) -> anyhow::Result<Option<Creature>> {
        match self.request(FrameBody::QueryCreature { guid }).await? {
            FrameBody::Creature { creature } => Ok(creature.map(Into::into)),
            other => anyhow::bail!("unexpected creature response: {other:?}"),
        }
    }

    pub async fn creature_by_entry(&self, entry: u32) -> anyhow::Result<Option<Creature>> {
        match self
            .request(FrameBody::QueryCreatureEntry { entry })
            .await?
        {
            FrameBody::Creature { creature } => Ok(creature.map(Into::into)),
            other => anyhow::bail!("unexpected creature response: {other:?}"),
        }
    }

    pub async fn gameobject(&self, guid: u64) -> anyhow::Result<Option<GameObject>> {
        match self.request(FrameBody::QueryGameObject { guid }).await? {
            FrameBody::GameObject { object } => Ok(object),
            other => anyhow::bail!("unexpected gameobject response: {other:?}"),
        }
    }

    pub async fn gameobject_by_entry(&self, entry: u32) -> anyhow::Result<Option<GameObject>> {
        match self
            .request(FrameBody::QueryGameObjectEntry { entry })
            .await?
        {
            FrameBody::GameObject { object } => Ok(object),
            other => anyhow::bail!("unexpected gameobject response: {other:?}"),
        }
    }

    pub async fn npc_text_pages(&self, text_id: u32) -> anyhow::Result<[NpcTextPage; 8]> {
        match self.request(FrameBody::QueryNpcText { text_id }).await? {
            FrameBody::NpcText { pages } => Ok(pages),
            other => anyhow::bail!("unexpected npc text response: {other:?}"),
        }
    }

    pub async fn quest(&self, entry: u32) -> anyhow::Result<Option<QuestRow>> {
        match self.request(FrameBody::QueryQuest { entry }).await? {
            FrameBody::Quest { quest } => Ok(quest),
            other => anyhow::bail!("unexpected quest response: {other:?}"),
        }
    }

    pub async fn questgiver_status(&self, player: u64, npc: u64) -> anyhow::Result<()> {
        self.request(FrameBody::QuestGiverStatus { player, npc })
            .await?;
        Ok(())
    }

    pub async fn open_questgiver(&self, player: u64, npc: u64) -> anyhow::Result<()> {
        self.request(FrameBody::QuestGiverHello { player, npc })
            .await?;
        Ok(())
    }

    pub async fn query_quest_details(
        &self,
        player: u64,
        npc: u64,
        quest_id: u32,
    ) -> anyhow::Result<()> {
        self.request(FrameBody::QuestGiverQuery {
            player,
            npc,
            quest_id,
        })
        .await?;
        Ok(())
    }

    pub async fn accept_quest(&self, player: u64, npc: u64, quest_id: u32) -> anyhow::Result<()> {
        self.request(FrameBody::QuestGiverAccept {
            player,
            npc,
            quest_id,
        })
        .await?;
        Ok(())
    }

    pub async fn complete_quest(&self, player: u64, npc: u64, quest_id: u32) -> anyhow::Result<()> {
        self.request(FrameBody::QuestGiverComplete {
            player,
            npc,
            quest_id,
        })
        .await?;
        Ok(())
    }

    pub async fn choose_quest_reward(
        &self,
        player: u64,
        npc: u64,
        quest_id: u32,
        reward: u32,
    ) -> anyhow::Result<()> {
        self.request(FrameBody::QuestGiverChooseReward {
            player,
            npc,
            quest_id,
            reward,
        })
        .await?;
        Ok(())
    }

    pub async fn abandon_quest(&self, player: u64, slot: u8) -> anyhow::Result<()> {
        self.request(FrameBody::QuestLogRemove { player, slot })
            .await?;
        Ok(())
    }
}

pub async fn serve_maps(bind: SocketAddr, maps: HashMap<u32, World>) -> anyhow::Result<()> {
    let listener = TcpListener::bind(bind).await?;
    serve_maps_listener(listener, maps).await
}

pub async fn serve_maps_listener(
    listener: TcpListener,
    maps: HashMap<u32, World>,
) -> anyhow::Result<()> {
    let bind = listener.local_addr()?;
    tracing::info!(%bind, maps = maps.len(), "map-server listening");
    tracing::info!(%bind, maps = maps.len(), "map-server ready");
    let maps = Arc::new(maps);
    loop {
        let (stream, peer) = listener.accept().await?;
        let maps = maps.clone();
        tokio::spawn(async move {
            if let Err(error) = handle_map_client(stream, maps).await {
                tracing::debug!(%peer, %error, "map rpc session ended");
            }
        });
    }
}

async fn handle_map_client(
    stream: TcpStream,
    maps: Arc<HashMap<u32, World>>,
) -> anyhow::Result<()> {
    let (mut reader, mut writer) = stream.into_split();
    let (event_tx, mut event_rx) = mpsc::unbounded_channel::<WorldEvent>();
    let (mailbox, mut local_rx) = PlayerMailbox::channel();
    tokio::spawn(async move {
        while let Some(event) = local_rx.recv().await {
            if event_tx.send(event).is_err() {
                break;
            }
        }
    });

    let mut joined: Option<(u32, u64)> = None;
    loop {
        tokio::select! {
            event = event_rx.recv() => {
                let Some(event) = event else { break };
                write_frame(
                    &mut writer,
                    &Frame {
                        id: 0,
                        body: FrameBody::Event(event_to_wire(event)),
                    },
                )
                .await?;
            }
            frame = read_frame(&mut reader) => {
                let frame = frame?;
                let body = handle_request(&maps, &mailbox, &mut joined, frame.body);
                write_frame(&mut writer, &Frame { id: frame.id, body }).await?;
            }
        }
    }

    if let Some((map_id, guid)) = joined {
        if let Some(world) = maps.get(&map_id) {
            world.leave(guid, &mailbox);
        }
    }
    Ok(())
}

fn handle_request(
    maps: &HashMap<u32, World>,
    mailbox: &PlayerMailbox,
    joined: &mut Option<(u32, u64)>,
    body: FrameBody,
) -> FrameBody {
    match body {
        FrameBody::Join { map_id, player } => {
            let Some(world) = maps.get(&map_id) else {
                return FrameBody::Error(format!("map-server does not host map {map_id}"));
            };
            if let Some((old_map, guid)) = joined.take() {
                if let Some(world) = maps.get(&old_map) {
                    world.leave(guid, mailbox);
                }
            }
            *joined = Some((map_id, player.guid));
            let others = world.join(player.clone(), mailbox.clone());
            FrameBody::JoinOk {
                others,
                creatures: world
                    .creatures_near(player.position)
                    .iter()
                    .map(WireCreature::from)
                    .collect(),
                gameobjects: world.gameobjects_near(player.position),
            }
        }
        FrameBody::Leave { guid } => {
            if let Some((map_id, _)) = *joined {
                if let Some(world) = maps.get(&map_id) {
                    world.leave(guid, mailbox);
                }
            }
            *joined = None;
            FrameBody::Ack
        }
        FrameBody::Move { guid, position } => with_world(maps, joined, |world| {
            world.broadcast_move(mailbox, Movement::without_packet(guid, position));
            FrameBody::Ack
        }),
        FrameBody::Speak { chat } => with_world(maps, joined, |world| {
            world.speak(mailbox, chat);
            FrameBody::Ack
        }),
        FrameBody::Attack { attacker, target } => with_world(maps, joined, |world| {
            world.start_attack(mailbox, attacker, target);
            FrameBody::Ack
        }),
        FrameBody::StopAttack { attacker } => with_world(maps, joined, |world| {
            world.stop_attack(mailbox, attacker);
            FrameBody::Ack
        }),
        FrameBody::StandState { guid, state } => with_world(maps, joined, |world| {
            world.change_stand_state(mailbox, guid, state);
            FrameBody::Ack
        }),
        FrameBody::GossipHello { player, npc } => with_world(maps, joined, |world| {
            world.open_gossip(mailbox, player, npc);
            FrameBody::Ack
        }),
        FrameBody::GossipSelect {
            player,
            npc,
            option,
        } => with_world(maps, joined, |world| {
            world.select_gossip_option(mailbox, player, npc, option);
            FrameBody::Ack
        }),
        FrameBody::ListVendor { player, npc } => with_world(maps, joined, |world| {
            world.list_vendor(mailbox, player, npc);
            FrameBody::Ack
        }),
        FrameBody::BuyItem {
            player,
            npc,
            item,
            amount,
        } => with_world(maps, joined, |world| {
            world.buy_item(mailbox, player, npc, item, amount);
            FrameBody::Ack
        }),
        FrameBody::OpenLoot { player, npc } => with_world(maps, joined, |world| {
            world.open_loot(mailbox, player, npc);
            FrameBody::Ack
        }),
        FrameBody::TakeLoot { player, index } => with_world(maps, joined, |world| {
            world.take_loot(mailbox, player, index);
            FrameBody::Ack
        }),
        FrameBody::CloseLoot { player } => with_world(maps, joined, |world| {
            world.close_loot(mailbox, player);
            FrameBody::Ack
        }),
        FrameBody::QueryItem { entry } => with_world(maps, joined, |world| FrameBody::Item {
            item: world.item(entry),
        }),
        FrameBody::QueryPlayer { guid } => with_world(maps, joined, |world| FrameBody::Player {
            player: world.player(guid),
        }),
        FrameBody::QueryCreature { guid } => {
            with_world(maps, joined, |world| FrameBody::Creature {
                creature: world.creature(guid).as_ref().map(WireCreature::from),
            })
        }
        FrameBody::QueryCreatureEntry { entry } => {
            with_world(maps, joined, |world| FrameBody::Creature {
                creature: world
                    .creature_by_entry(entry)
                    .as_ref()
                    .map(WireCreature::from),
            })
        }
        FrameBody::QueryGameObject { guid } => {
            with_world(maps, joined, |world| FrameBody::GameObject {
                object: world.gameobject(guid),
            })
        }
        FrameBody::QueryGameObjectEntry { entry } => {
            with_world(maps, joined, |world| FrameBody::GameObject {
                object: world.gameobject_by_entry(entry),
            })
        }
        FrameBody::QueryNpcText { text_id } => {
            with_world(maps, joined, |world| FrameBody::NpcText {
                pages: world.npc_text_pages(text_id),
            })
        }
        FrameBody::QueryQuest { entry } => with_world(maps, joined, |world| FrameBody::Quest {
            quest: world.quest(entry),
        }),
        FrameBody::QuestGiverStatus { player, npc } => with_world(maps, joined, |world| {
            world.questgiver_status(mailbox, player, npc);
            FrameBody::Ack
        }),
        FrameBody::QuestGiverHello { player, npc } => with_world(maps, joined, |world| {
            world.open_questgiver(mailbox, player, npc);
            FrameBody::Ack
        }),
        FrameBody::QuestGiverQuery {
            player,
            npc,
            quest_id,
        } => with_world(maps, joined, |world| {
            world.query_quest_details(mailbox, player, npc, quest_id);
            FrameBody::Ack
        }),
        FrameBody::QuestGiverAccept {
            player,
            npc,
            quest_id,
        } => with_world(maps, joined, |world| {
            world.accept_quest(mailbox, player, npc, quest_id);
            FrameBody::Ack
        }),
        FrameBody::QuestGiverComplete {
            player,
            npc,
            quest_id,
        } => with_world(maps, joined, |world| {
            world.complete_quest(mailbox, player, npc, quest_id);
            FrameBody::Ack
        }),
        FrameBody::QuestGiverChooseReward {
            player,
            npc,
            quest_id,
            reward,
        } => with_world(maps, joined, |world| {
            world.choose_quest_reward(mailbox, player, npc, quest_id, reward);
            FrameBody::Ack
        }),
        FrameBody::QuestLogRemove { player, slot } => with_world(maps, joined, |world| {
            world.abandon_quest(mailbox, player, slot);
            FrameBody::Ack
        }),
        other => FrameBody::Error(format!("not a request: {other:?}")),
    }
}

fn with_world(
    maps: &HashMap<u32, World>,
    joined: &Option<(u32, u64)>,
    f: impl FnOnce(&World) -> FrameBody,
) -> FrameBody {
    let Some((map_id, _)) = joined else {
        return FrameBody::Error("not in world".into());
    };
    let Some(world) = maps.get(map_id) else {
        return FrameBody::Error("unknown map".into());
    };
    f(world)
}

pub fn spawn_map_tickers(maps: &HashMap<u32, World>) {
    for world in maps.values() {
        let ticker = world.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_millis(200));
            loop {
                interval.tick().await;
                ticker.tick(Instant::now());
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wow_shared::MAP_EASTERN_KINGDOMS;

    #[tokio::test]
    async fn remote_join_returns_northshire_npcs() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let mut maps = HashMap::new();
        maps.insert(MAP_EASTERN_KINGDOMS, World::new());
        tokio::spawn(async move {
            serve_maps_listener(listener, maps).await.unwrap();
        });
        let (mailbox, _rx) = PlayerMailbox::channel();
        let session = MapSession::connect(addr, MAP_EASTERN_KINGDOMS, mailbox)
            .await
            .unwrap();
        let player = Player::new(1, "User1", Position::NORTHSHIRE);
        let (others, creatures, _gameobjects) = session.join(player).await.unwrap();
        assert!(others.is_empty());
        assert!(!creatures.is_empty());
    }
}
