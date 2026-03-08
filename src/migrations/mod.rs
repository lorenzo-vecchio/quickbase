pub mod initial;

use crate::db::migrations::MigrationsList;

pub fn register_system_migrations(list: &mut MigrationsList) {
    list.register(initial::migration());
}