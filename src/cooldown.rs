use std::{
    collections::HashMap,
    time::{Duration, Instant},
};

const CAP: usize = 20000;
#[derive(Default)]
pub struct Cooldowns {
    users: HashMap<(u64, u64), Instant>,
    guilds: HashMap<u64, Instant>,
    messages: HashMap<u64, Instant>,
    texts: HashMap<(u64, String), Instant>,
}
impl Cooldowns {
    pub fn prune(&mut self, now: Instant) {
        self.users.retain(|_, until| *until > now);
        self.guilds.retain(|_, until| *until > now);
        self.messages.retain(|_, until| *until > now);
        self.texts.retain(|_, until| *until > now);
    }
    pub fn claim(
        &mut self,
        guild: u64,
        user: u64,
        message: u64,
        text: String,
        seconds: u64,
        now: Instant,
    ) -> bool {
        self.prune(now);
        if self.users.contains_key(&(guild, user))
            || self.guilds.contains_key(&guild)
            || self.messages.contains_key(&message)
            || self.texts.contains_key(&(guild, text.clone()))
            || self.messages.len() >= CAP
            || self.users.len() >= CAP
            || self.texts.len() >= CAP
        {
            return false;
        }
        self.users
            .insert((guild, user), now + Duration::from_secs(seconds.min(86400)));
        self.guilds.insert(guild, now + Duration::from_secs(3));
        self.messages
            .insert(message, now + Duration::from_secs(600));
        self.texts
            .insert((guild, text), now + Duration::from_secs(300));
        true
    }
}
