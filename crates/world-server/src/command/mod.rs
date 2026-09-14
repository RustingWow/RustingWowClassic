mod account;
mod gm;
mod handlers;
mod lookup;
mod movement;
mod npc;
mod parse;
mod player;
mod server;
mod services;
mod table;
mod tele;

use wow_shared::{Account, Position};

pub use parse::is_command;
pub use services::{AuthGmClient, CommandServices};
pub use tele::TeleStore;

use crate::catalog::Catalog;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum Security {
    Player = 0,
    Moderator = 1,
    GameMaster = 2,
    Admin = 3,
}

pub struct Command {
    pub name: &'static str,
    pub aliases: &'static [&'static str],
    pub security: Security,
    pub help: &'static str,
    pub handler: Handler,
}

pub enum Handler {
    Live(fn(&CommandCtx<'_>, &[&str]) -> CommandResult),
    Nested {
        children: &'static [Command],
        default: Option<fn(&CommandCtx<'_>, &[&str]) -> CommandResult>,
    },
    Stub,
}

pub struct CreatureInfo {
    pub guid: u64,
    pub entry: u32,
    pub name: String,
    pub position: Position,
    pub health: i32,
    pub max_health: i32,
    pub level: i32,
    pub faction: i32,
    pub hostile: bool,
}

pub struct OnlineInfo {
    pub name: String,
    pub map_id: u32,
    pub gmlevel: u8,
}

pub struct CommandCtx<'a> {
    pub account: &'a Account,
    pub character_name: &'a str,
    pub character_guid: u64,
    pub map_id: u32,
    pub position: Position,
    pub copper: u32,
    pub health: i32,
    pub max_health: i32,
    pub selection: Option<u64>,
    pub selected_creature: Option<CreatureInfo>,
    pub selected_player_name: Option<String>,
    pub gm_on: bool,
    pub gm_visible: bool,
    pub gm_chat: bool,
    pub catalog: Option<&'a Catalog>,
    pub tele: &'a TeleStore,
    pub online: &'a [OnlineInfo],
    pub motd: &'a str,
    pub player_count: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Effect {
    Teleport {
        map_id: u32,
        position: Position,
    },
    Appear {
        name: String,
    },
    Summon {
        name: String,
    },
    Kick,
    Announce {
        text: String,
        named: bool,
        gm_only: bool,
        notify: bool,
    },
    SetGmOn(bool),
    SetGmVisible(bool),
    SetGmChat(bool),
    ModifyMoney(i64),
    Die {
        guid: Option<u64>,
    },
    Revive {
        guid: Option<u64>,
    },
    SetHp {
        health: i32,
    },
    AddItem {
        entry: u32,
        count: u32,
    },
    SpawnNpc {
        entry: u32,
    },
    DeleteNpc {
        guid: u64,
    },
    Save,
    Recall,
    SetGmLevel {
        username: String,
        level: u8,
    },
    TeleAdd {
        name: String,
    },
    TeleDel {
        name: String,
    },
    SetMotd(String),
    GmList,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CommandResult {
    pub messages: Vec<String>,
    pub effects: Vec<Effect>,
}

impl CommandResult {
    pub fn ok(message: impl Into<String>) -> Self {
        Self {
            messages: vec![message.into()],
            effects: Vec::new(),
        }
    }

    pub fn many(messages: Vec<String>) -> Self {
        Self {
            messages,
            effects: Vec::new(),
        }
    }

    pub fn err(message: impl Into<String>) -> Self {
        Self::ok(message)
    }

    pub fn with(mut self, effect: Effect) -> Self {
        self.effects.push(effect);
        self
    }
}

pub fn dispatch(text: &str, ctx: &CommandCtx<'_>) -> CommandResult {
    let tokens = parse::tokenize(text);
    if tokens.is_empty() {
        return CommandResult::err("There is no such command.");
    }
    let refs: Vec<&str> = tokens.iter().map(String::as_str).collect();
    dispatch_in(table::ROOT, ctx.account.gmlevel, &refs, ctx)
}

fn dispatch_in(
    commands: &[Command],
    gmlevel: u8,
    tokens: &[&str],
    ctx: &CommandCtx<'_>,
) -> CommandResult {
    if tokens.is_empty() {
        return list_available(commands, gmlevel);
    }
    let Some(command) = find(commands, tokens[0]) else {
        return CommandResult::err("There is no such command.");
    };
    if gmlevel < command.security as u8 {
        return CommandResult::err("You do not have access to this command.");
    }
    match command.handler {
        Handler::Live(handler) => handler(ctx, &tokens[1..]),
        Handler::Stub => CommandResult::err("Command not implemented yet."),
        Handler::Nested { children, default } => {
            if tokens.len() == 1 {
                if let Some(handler) = default {
                    return handler(ctx, &[]);
                }
                return list_available(children, gmlevel);
            }
            if find(children, tokens[1]).is_some() {
                return dispatch_in(children, gmlevel, &tokens[1..], ctx);
            }
            if let Some(handler) = default {
                return handler(ctx, &tokens[1..]);
            }
            dispatch_in(children, gmlevel, &tokens[1..], ctx)
        }
    }
}

fn find<'a>(commands: &'a [Command], name: &str) -> Option<&'a Command> {
    commands.iter().find(|command| {
        command.name.eq_ignore_ascii_case(name)
            || command
                .aliases
                .iter()
                .any(|alias| alias.eq_ignore_ascii_case(name))
    })
}

fn list_available(commands: &[Command], gmlevel: u8) -> CommandResult {
    let names: Vec<_> = commands
        .iter()
        .filter(|command| gmlevel >= command.security as u8)
        .map(|command| command.name.to_string())
        .collect();
    if names.is_empty() {
        return CommandResult::err("You do not have access to this command.");
    }
    CommandResult::ok(format!("Commands: {}", names.join(", ")))
}

pub(crate) fn help_tree(path: &[&str], gmlevel: u8, remaining: &[&str]) -> CommandResult {
    help_in(table::ROOT, path, gmlevel, remaining)
}

fn help_in(commands: &[Command], path: &[&str], gmlevel: u8, remaining: &[&str]) -> CommandResult {
    if remaining.is_empty() {
        let mut lines = Vec::new();
        for command in commands {
            if gmlevel < command.security as u8 {
                continue;
            }
            lines.push(format!(".{} — {}", command.name, command.help));
        }
        if lines.is_empty() {
            return CommandResult::err("No commands available.");
        }
        return CommandResult::many(lines);
    }
    let Some(command) = find(commands, remaining[0]) else {
        return CommandResult::err("There is no such command.");
    };
    if gmlevel < command.security as u8 {
        return CommandResult::err("You do not have access to this command.");
    }
    if remaining.len() == 1 {
        let mut lines = vec![format!(".{} — {}", command.name, command.help)];
        if let Handler::Nested { children, .. } = command.handler {
            for child in children {
                if gmlevel >= child.security as u8 {
                    lines.push(format!("  {} — {}", child.name, child.help));
                }
            }
        }
        return CommandResult::many(lines);
    }
    match command.handler {
        Handler::Nested { children, .. } => help_in(children, path, gmlevel, &remaining[1..]),
        _ => CommandResult::ok(format!(".{} — {}", command.name, command.help)),
    }
}

pub(crate) fn parse_id(value: &str) -> Option<u32> {
    value.parse().ok()
}

pub(crate) const fn live(
    name: &'static str,
    aliases: &'static [&'static str],
    security: Security,
    help: &'static str,
    handler: fn(&CommandCtx<'_>, &[&str]) -> CommandResult,
) -> Command {
    Command {
        name,
        aliases,
        security,
        help,
        handler: Handler::Live(handler),
    }
}

pub(crate) const fn stub(
    name: &'static str,
    aliases: &'static [&'static str],
    security: Security,
    help: &'static str,
) -> Command {
    Command {
        name,
        aliases,
        security,
        help,
        handler: Handler::Stub,
    }
}

pub(crate) const fn nested(
    name: &'static str,
    aliases: &'static [&'static str],
    security: Security,
    help: &'static str,
    children: &'static [Command],
    default: Option<fn(&CommandCtx<'_>, &[&str]) -> CommandResult>,
) -> Command {
    Command {
        name,
        aliases,
        security,
        help,
        handler: Handler::Nested { children, default },
    }
}

#[cfg(test)]
#[path = "../../test/command.rs"]
mod tests;
