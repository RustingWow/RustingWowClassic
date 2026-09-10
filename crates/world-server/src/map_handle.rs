use std::time::Instant;

use wow_shared::Position;

use crate::creature::Creature;
use crate::player::Player;
use crate::world::{Chat, Movement, PlayerMailbox, World};

/// Authority for a single map: presence, NPCs, combat tick.
/// `World` is the in-process implementation; `rpc::MapSession` is the remote one.
pub trait MapHandle {
    fn map_id(&self) -> u32;
    fn join(&self, player: Player, mailbox: PlayerMailbox) -> Vec<Player>;
    fn leave(&self, guid: u64, mailbox: &PlayerMailbox);
    fn player(&self, guid: u64) -> Option<Player>;
    fn creatures(&self) -> Vec<Creature>;
    fn creatures_near(&self, position: Position) -> Vec<Creature>;
    fn creature(&self, guid: u64) -> Option<Creature>;
    fn creature_by_entry(&self, entry: u32) -> Option<Creature>;
    fn broadcast_move(&self, from: &PlayerMailbox, movement: Movement);
    fn change_stand_state(&self, from: &PlayerMailbox, guid: u64, state: u8);
    fn open_gossip(&self, from: &PlayerMailbox, player: u64, npc: u64);
    fn select_gossip_option(&self, from: &PlayerMailbox, player: u64, npc: u64, option_id: u32);
    fn list_vendor(&self, from: &PlayerMailbox, player: u64, npc: u64);
    fn buy_item(&self, from: &PlayerMailbox, player: u64, npc: u64, item_id: u32, amount: u32);
    fn open_loot(&self, from: &PlayerMailbox, player: u64, npc: u64);
    fn take_loot(&self, from: &PlayerMailbox, player: u64, npc: u64, index: u8);
    fn close_loot(&self, from: &PlayerMailbox, player: u64);
    fn npc_text(&self, text_id: u32) -> Option<String>;
    fn item(&self, entry: u32) -> Option<crate::catalog::ItemRow>;
    fn speak(&self, from: &PlayerMailbox, chat: Chat);
    fn start_attack(&self, from: &PlayerMailbox, attacker: u64, target: u64);
    fn stop_attack(&self, from: &PlayerMailbox, attacker: u64);
    fn tick(&self, now: Instant);
}

impl MapHandle for World {
    fn map_id(&self) -> u32 {
        World::map_id(self)
    }

    fn join(&self, player: Player, mailbox: PlayerMailbox) -> Vec<Player> {
        World::join(self, player, mailbox)
    }

    fn leave(&self, guid: u64, mailbox: &PlayerMailbox) {
        World::leave(self, guid, mailbox)
    }

    fn player(&self, guid: u64) -> Option<Player> {
        World::player(self, guid)
    }

    fn creatures(&self) -> Vec<Creature> {
        World::creatures(self)
    }

    fn creatures_near(&self, position: Position) -> Vec<Creature> {
        World::creatures_near(self, position)
    }

    fn creature(&self, guid: u64) -> Option<Creature> {
        World::creature(self, guid)
    }

    fn creature_by_entry(&self, entry: u32) -> Option<Creature> {
        World::creature_by_entry(self, entry)
    }

    fn broadcast_move(&self, from: &PlayerMailbox, movement: Movement) {
        World::broadcast_move(self, from, movement)
    }

    fn change_stand_state(&self, from: &PlayerMailbox, guid: u64, state: u8) {
        World::change_stand_state(self, from, guid, state)
    }

    fn open_gossip(&self, from: &PlayerMailbox, player: u64, npc: u64) {
        World::open_gossip(self, from, player, npc)
    }

    fn select_gossip_option(&self, from: &PlayerMailbox, player: u64, npc: u64, option_id: u32) {
        World::select_gossip_option(self, from, player, npc, option_id)
    }

    fn list_vendor(&self, from: &PlayerMailbox, player: u64, npc: u64) {
        World::list_vendor(self, from, player, npc)
    }

    fn buy_item(&self, from: &PlayerMailbox, player: u64, npc: u64, item_id: u32, amount: u32) {
        World::buy_item(self, from, player, npc, item_id, amount)
    }

    fn open_loot(&self, from: &PlayerMailbox, player: u64, npc: u64) {
        World::open_loot(self, from, player, npc)
    }

    fn take_loot(&self, from: &PlayerMailbox, player: u64, npc: u64, index: u8) {
        let _ = npc;
        World::take_loot(self, from, player, index)
    }

    fn close_loot(&self, from: &PlayerMailbox, player: u64) {
        World::close_loot(self, from, player)
    }

    fn npc_text(&self, text_id: u32) -> Option<String> {
        World::npc_text(self, text_id)
    }

    fn item(&self, entry: u32) -> Option<crate::catalog::ItemRow> {
        World::item(self, entry)
    }

    fn speak(&self, from: &PlayerMailbox, chat: Chat) {
        World::speak(self, from, chat)
    }

    fn start_attack(&self, from: &PlayerMailbox, attacker: u64, target: u64) {
        World::start_attack(self, from, attacker, target)
    }

    fn stop_attack(&self, from: &PlayerMailbox, attacker: u64) {
        World::stop_attack(self, from, attacker)
    }

    fn tick(&self, now: Instant) {
        World::tick(self, now)
    }
}
