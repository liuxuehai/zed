use client::{Client, UserStore};
use gpui::{App, Entity};

pub fn init(_client: Arc<Client>, _user_store: Entity<UserStore>, _cx: &mut App) {
    // Edit prediction registry disabled in this build.
}

use std::sync::Arc;
