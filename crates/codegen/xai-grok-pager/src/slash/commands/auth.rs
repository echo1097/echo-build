//! `/auth` -- manage the current OpenRouter API key.

use crate::app::actions::Action;
use crate::slash::command::{CommandExecCtx, CommandResult, SlashCommand};

pub struct AuthCommand;

impl SlashCommand for AuthCommand {
    fn name(&self) -> &str {
        "auth"
    }

    fn description(&self) -> &str {
        "Manage the current OpenRouter API key"
    }

    fn session_scoped(&self) -> bool {
        true
    }

    fn usage(&self) -> &str {
        "/auth"
    }

    fn run(&self, _ctx: &mut CommandExecCtx, _args: &str) -> CommandResult {
        CommandResult::Action(Action::ShowAuthManagement)
    }
}
