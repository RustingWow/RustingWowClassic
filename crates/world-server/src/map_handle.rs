use std::time::Instant;

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
    fn creature(&self, guid: u64) -> Option<Creature>;
    fn creature_by_entry(&self, entry: u32) -> Option<Creature>;
    fn broadcast_move(&self, from: &PlayerMailbox, movement: Movement);
    fn change_stand_state(&self, from: &PlayerMailbox, guid: u64, state: u8);
    fn open_gossip(&self, from: &PlayerMailbox, player: u64, npc: u64);
    fn select_gossip_option(&self, from: &PlayerMailbox, player: u64, npc: u64, option_id: u32);
    fn npc_text(&self, text_id: u32) -> Option<String>;
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

    fn npc_text(&self, text_id: u32) -> Option<String> {
        World::npc_text(self, text_id)
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
