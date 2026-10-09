use std::sync::PoisonError;

use pumpkin_util::PermissionLvl;
use pumpkin_util::permission::{Permission, PermissionDefault, PermissionRegistry};
use pumpkin_util::text::TextComponent;

use crate::command::argument_builder::{ArgumentBuilder, command};
use crate::command::context::command_context::CommandContext;
use crate::command::node::dispatcher::CommandDispatcher;
use crate::command::node::{CommandExecutor, CommandExecutorResult};
use crate::server::tps_bar::{self, TPS_BAR_UUID};

const DESCRIPTION: &str = "Toggles a boss bar showing TPS, MSPT, tick utilization and RAM.";
const PERMISSION: &str = "pumpkin:command.tpsbar";

struct TpsBarExecutor;

impl CommandExecutor for TpsBarExecutor {
    fn execute(&self, context: &CommandContext) -> CommandExecutorResult {
        let player = context.source.player_or_err()?;
        let mut shown = player
            .tps_bar
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        *shown = !*shown;
        let enabled = *shown;

        if enabled {
            player.send_bossbar(&tps_bar::bossbar(context.server()));
        } else {
            player.remove_bossbar(TPS_BAR_UUID);
        }
        drop(shown);

        let state = if enabled { "on" } else { "off" };
        context.source.send_feedback(
            TextComponent::text(format!("TPS bar turned {state}")),
            false,
        );

        Ok(i32::from(enabled))
    }
}

pub fn register(dispatcher: &mut CommandDispatcher, registry: &PermissionRegistry) {
    registry.register_permission_or_panic(Permission::new(
        PERMISSION,
        DESCRIPTION,
        PermissionDefault::Op(PermissionLvl::Two),
    ));

    dispatcher.register(
        command("tpsbar", DESCRIPTION)
            .requires(PERMISSION)
            .executes(TpsBarExecutor),
    );
}
