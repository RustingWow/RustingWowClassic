use super::{CommandCtx, CommandResult, help_tree};

pub use super::account::{account_set_gmlevel, account_show};
pub use super::gm::{gm_chat, gm_ingame, gm_list, gm_off, gm_on, gm_toggle, gm_visible};
pub use super::lookup::{lookup_creature, lookup_item, lookup_tele};
pub use super::movement::{appear, go_xyz, gps, recall, summon, tele, tele_add, tele_del};
pub use super::npc::{npc_add_temp, npc_delete, npc_info};
pub use super::player::{
    additem, announce, die, gmannounce, guid, kick, modify_hp, modify_money, nameannounce, notify,
    pinfo, revive, save,
};
pub use super::server::{server_info, server_motd, server_set_motd};

pub fn unimplemented(_ctx: &CommandCtx<'_>, _args: &[&str]) -> CommandResult {
    CommandResult::err("Command not implemented yet.")
}

pub fn help(ctx: &CommandCtx<'_>, args: &[&str]) -> CommandResult {
    help_tree(&[], ctx.account.gmlevel, args)
}

pub fn commands(ctx: &CommandCtx<'_>, _args: &[&str]) -> CommandResult {
    help_tree(&[], ctx.account.gmlevel, &[])
}
