mod commands;
mod handler;
mod rating_range;
mod role_manager;
mod run;

use crate::{db::Pool, lichess};
use role_manager::RoleManager;
pub use run::run;

struct Data {
    pool: Pool,
    lichess: lichess::Client,
    roles: RoleManager,
}
