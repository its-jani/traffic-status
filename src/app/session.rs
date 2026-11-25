//! Session bookkeeping, decoupled from egui.
//!
//! `SessionStore` is the daemon's multi-agent session registry. Concurrent CLI
//! sessions (Qoder, OpenCode, Codex, …) each own one entry, and the displayed
//! bulb is the strict severity maximum across all of them
//! (`Red > Yellow > Green`), so one blocked agent can never be masked by
//! another reporting `Green`.
//!
//! The map is only mutated from the UI thread — IPC commands arrive through a
//! thread-safe channel — so it needs no lock of its own. It applies commands
//! and returns the window-level [`AppEffect`]s the UI should perform, so the
//! state machine can be tested without a window.

use std::collections::HashMap;
use std::time::{Duration, Instant};

use crate::types::{IpcCommand, LightState, SessionEntry};

/// Sessions that have not sent an update for this long are garbage collected.
pub const SESSION_TTL: Duration = Duration::from_secs(20 * 60);

/// Window/process side effects the UI must perform after a command.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppEffect {
    Repaint,
    Unminimize,
    FocusWindow,
    CloseWindow,
    ExitProcess,
}

pub struct SessionStore {
    sessions: HashMap<String, SessionEntry>,
    order: Vec<String>,
    active: Option<String>,
}

impl SessionStore {
    /// Fresh store containing the "agent-1" placeholder session.
    pub fn with_default_session() -> Self {
        let id = "agent-1".to_string();
        let mut sessions = HashMap::new();
        sessions.insert(
            id.clone(),
            SessionEntry::new(
                id.clone(),
                Some("Session #1".to_string()),
                None,
                Some(LightState::Green),
            ),
        );
        Self {
            sessions,
            order: vec![id.clone()],
            active: Some(id),
        }
    }

    pub fn sessions(&self) -> &HashMap<String, SessionEntry> {
        &self.sessions
    }

    pub fn order(&self) -> &[String] {
        &self.order
    }

    pub fn active_id(&self) -> Option<&str> {
        self.active.as_deref()
    }

    pub fn active_session(&self) -> Option<&SessionEntry> {
        self.active.as_ref().and_then(|id| self.sessions.get(id))
    }

    pub fn select(&mut self, id: &str) {
        self.active = Some(id.to_string());
    }

    /// Fall back to the first tab when the active one is gone.
    pub fn ensure_active(&mut self) {
        if self.active.is_none() {
            self.active = self.order.first().cloned();
        }
    }

    pub fn is_empty(&self) -> bool {
        self.sessions.is_empty()
    }

    /// Bulb actually shown on the hardware: the highest severity across every
    /// live session. An empty registry resolves to [`LightState::Off`].
    pub fn resolved_state(&self) -> LightState {
        self.sessions
            .values()
            .map(|s| s.state)
            .max_by_key(|s| s.severity())
            .unwrap_or(LightState::Off)
    }

    /// Continuous repaint is required while the bulb pulses (yellow work,
    /// red attention) or while the color weights are still blending.
    pub fn has_active_animation(&self) -> bool {
        matches!(self.resolved_state(), LightState::Yellow | LightState::Red)
    }

    /// Drop the placeholder once a real session shows up.
    fn replace_placeholder(&mut self, session_id: &str) {
        if self.sessions.len() == 1
            && self.sessions.contains_key("agent-1")
            && session_id != "agent-1"
        {
            self.sessions.remove("agent-1");
            self.order.retain(|id| id != "agent-1");
        }
    }

    fn push_order_if_missing(&mut self, session_id: &str) {
        if !self.order.iter().any(|id| id == session_id) {
            self.order.push(session_id.to_string());
        }
    }

    /// Remove one session and re-target the active tab if it pointed at it.
    fn remove_session(&mut self, session_id: &str) -> bool {
        let removed = self.sessions.remove(session_id).is_some();
        if removed {
            self.order.retain(|id| id != session_id);
            if self.active.as_deref() == Some(session_id) {
                self.active = self.order.last().cloned();
            }
        }
        removed
    }

    /// The widget has nothing left to show once the last session disappears.
    fn removal_effects(&self) -> Vec<AppEffect> {
        if self.sessions.is_empty() {
            vec![AppEffect::CloseWindow, AppEffect::ExitProcess]
        } else {
            vec![AppEffect::Repaint]
        }
    }

    /// Prune entries whose last update is older than `ttl`, evaluated at `now`.
    /// Returns the identifiers that were dropped (deterministic, so the caller
    /// can decide whether to repaint or close).
    pub fn prune_stale_at(&mut self, now: Instant, ttl: Duration) -> Vec<String> {
        let expired: Vec<String> = self
            .sessions
            .iter()
            .filter(|(id, entry)| {
                // The idle placeholder is not an agent session; keeping it warm
                // leaves the daemon (and its saved coordinates) alive.
                let is_lone_placeholder = id.as_str() == "agent-1" && self.sessions.len() == 1;
                !is_lone_placeholder && now.duration_since(entry.last_updated) >= ttl
            })
            .map(|(id, _)| id.clone())
            .collect();

        for id in &expired {
            self.remove_session(id);
        }

        expired
    }

    /// Prune against the wall clock and translate the result into UI effects.
    pub fn prune_stale(&mut self, ttl: Duration) -> Vec<AppEffect> {
        let expired = self.prune_stale_at(Instant::now(), ttl);
        if expired.is_empty() {
            return Vec::new();
        }
        if self.order.is_empty() {
            // Every real session went stale: fall back to the idle placeholder
            // instead of leaving an empty window behind.
            *self = SessionStore::with_default_session();
        }
        vec![AppEffect::Repaint]
    }

    pub fn apply(&mut self, cmd: IpcCommand) -> Vec<AppEffect> {
        match cmd {
            IpcCommand::SetState {
                session_id,
                state,
                label,
                source,
                message,
            } => {
                // `off` from a live session frees its slot immediately.
                if state == LightState::Off {
                    self.remove_session(&session_id);
                    return self.removal_effects();
                }

                self.replace_placeholder(&session_id);

                if let Some(session) = self.sessions.get_mut(&session_id) {
                    session.state = state;
                    if let Some(lbl) = label {
                        session.label = lbl;
                    }
                    if source.is_some() {
                        session.source = source;
                    }
                    if message.is_some() {
                        session.message = message;
                    }
                    session.last_updated = Instant::now();
                } else {
                    let mut session =
                        SessionEntry::new(session_id.clone(), label, source, Some(state));
                    session.message = message;
                    self.sessions.insert(session_id.clone(), session);
                    self.push_order_if_missing(&session_id);
                }
                self.active = Some(session_id);
                Vec::new()
            }
            IpcCommand::SessionOn {
                session_id,
                label,
                source,
                initial_state,
            } => {
                self.replace_placeholder(&session_id);

                if let Some(session) = self.sessions.get_mut(&session_id) {
                    if let Some(lbl) = label {
                        session.label = lbl;
                    }
                    if source.is_some() {
                        session.source = source;
                    }
                    if let Some(st) = initial_state {
                        session.state = st;
                    }
                    session.last_updated = Instant::now();
                } else {
                    let session =
                        SessionEntry::new(session_id.clone(), label, source, initial_state);
                    self.sessions.insert(session_id.clone(), session);
                    self.push_order_if_missing(&session_id);
                }
                self.active = Some(session_id);
                vec![
                    AppEffect::Unminimize,
                    AppEffect::FocusWindow,
                    AppEffect::Repaint,
                ]
            }
            IpcCommand::SessionOff { session_id } => {
                self.remove_session(&session_id);
                self.removal_effects()
            }
            IpcCommand::UpdateConfig { .. } => Vec::new(),
            IpcCommand::ClearAll => {
                self.sessions.clear();
                self.order.clear();
                self.active = None;
                vec![AppEffect::CloseWindow, AppEffect::ExitProcess]
            }
            IpcCommand::Shutdown => vec![AppEffect::CloseWindow, AppEffect::ExitProcess],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn store() -> SessionStore {
        SessionStore::with_default_session()
    }

    fn set(s: &mut SessionStore, id: &str, state: LightState) {
        s.apply(IpcCommand::SetState {
            session_id: id.into(),
            state,
            label: None,
            source: None,
            message: None,
        });
    }

    #[test]
    fn starts_with_placeholder() {
        let s = store();
        assert_eq!(s.order(), ["agent-1"]);
        assert_eq!(s.active_id(), Some("agent-1"));
        assert!(s.sessions().contains_key("agent-1"));
    }

    #[test]
    fn first_real_session_replaces_placeholder() {
        let mut s = store();
        let effects = s.apply(IpcCommand::SetState {
            session_id: "session-1".into(),
            state: LightState::Red,
            label: None,
            source: None,
            message: Some("boom".into()),
        });
        assert!(effects.is_empty());
        assert!(!s.sessions().contains_key("agent-1"));
        assert_eq!(s.order(), ["session-1"]);
        assert_eq!(s.active_id(), Some("session-1"));
        assert_eq!(s.sessions()["session-1"].message.as_deref(), Some("boom"));
    }

    #[test]
    fn session_on_inserts_and_requests_focus() {
        let mut s = store();
        let effects = s.apply(IpcCommand::SessionOn {
            session_id: "session-2".into(),
            label: Some("Builder".into()),
            source: Some("OpenCode".into()),
            initial_state: Some(LightState::Yellow),
        });
        assert_eq!(
            effects,
            vec![
                AppEffect::Unminimize,
                AppEffect::FocusWindow,
                AppEffect::Repaint
            ]
        );
        assert_eq!(s.sessions()["session-2"].label, "Builder");
        assert_eq!(
            s.sessions()["session-2"].source.as_deref(),
            Some("OpenCode")
        );
        assert!(s.has_active_animation());
    }

    #[test]
    fn session_off_removes_and_retargets_active() {
        let mut s = store();
        s.apply(IpcCommand::SessionOn {
            session_id: "a".into(),
            label: None,
            source: None,
            initial_state: None,
        });
        s.apply(IpcCommand::SessionOn {
            session_id: "b".into(),
            label: None,
            source: None,
            initial_state: None,
        });
        assert_eq!(s.active_id(), Some("b"));

        let effects = s.apply(IpcCommand::SessionOff {
            session_id: "b".into(),
        });
        assert_eq!(effects, vec![AppEffect::Repaint]);
        assert_eq!(s.active_id(), Some("a"));
        assert_eq!(s.order(), ["a"]);
    }

    #[test]
    fn removing_last_session_clears_active() {
        let mut s = store();
        let effects = s.apply(IpcCommand::SessionOff {
            session_id: "agent-1".into(),
        });
        assert_eq!(s.active_id(), None);
        assert!(s.order().is_empty());
        assert!(s.active_session().is_none());
        assert_eq!(
            effects,
            vec![AppEffect::CloseWindow, AppEffect::ExitProcess]
        );
    }

    #[test]
    fn forwarding_off_state_removes_the_session() {
        let mut s = store();
        set(&mut s, "one", LightState::Yellow);
        set(&mut s, "two", LightState::Green);
        assert_eq!(s.order().len(), 2);

        // A session that reports `off` must not linger as a stale yellow light.
        let effects = s.apply(IpcCommand::SetState {
            session_id: "one".into(),
            state: LightState::Off,
            label: None,
            source: None,
            message: None,
        });
        assert_eq!(effects, vec![AppEffect::Repaint]);
        assert!(!s.sessions().contains_key("one"));
        assert_eq!(s.resolved_state(), LightState::Green);
    }

    #[test]
    fn clear_all_empties_everything() {
        let mut s = store();
        let effects = s.apply(IpcCommand::ClearAll);
        assert!(s.sessions().is_empty());
        assert!(s.order().is_empty());
        assert_eq!(s.active_id(), None);
        assert!(!s.has_active_animation());
        assert_eq!(
            effects,
            vec![AppEffect::CloseWindow, AppEffect::ExitProcess]
        );
    }

    #[test]
    fn ensuring_active_picks_first_tab() {
        let mut s = store();
        s.apply(IpcCommand::ClearAll);
        set(&mut s, "x", LightState::Green);
        s.active = None;
        s.ensure_active();
        assert_eq!(s.active_id(), Some("x"));
    }

    #[test]
    fn shutdown_asks_ui_to_close_then_exit() {
        let mut s = store();
        assert_eq!(
            s.apply(IpcCommand::Shutdown),
            vec![AppEffect::CloseWindow, AppEffect::ExitProcess]
        );
    }

    #[test]
    fn empty_registry_resolves_to_off() {
        let mut s = store();
        s.apply(IpcCommand::ClearAll);
        assert_eq!(s.resolved_state(), LightState::Off);
        assert!(!s.has_active_animation());
    }

    #[test]
    fn red_outranks_yellow_and_green() {
        let mut s = store();
        set(&mut s, "qoder", LightState::Green);
        set(&mut s, "opencode", LightState::Yellow);
        assert_eq!(s.resolved_state(), LightState::Yellow);

        set(&mut s, "codex", LightState::Red);
        assert_eq!(s.resolved_state(), LightState::Red);

        // Another agent finishing green must not mask the blocked one.
        set(&mut s, "opencode", LightState::Green);
        set(&mut s, "qoder", LightState::Green);
        assert_eq!(s.resolved_state(), LightState::Red);
        assert!(s.has_active_animation());

        // Only clearing the red session releases the bulb.
        set(&mut s, "codex", LightState::Green);
        assert_eq!(s.resolved_state(), LightState::Green);
        assert!(!s.has_active_animation());
    }

    #[test]
    fn yellow_outranks_green() {
        let mut s = store();
        set(&mut s, "a", LightState::Green);
        set(&mut s, "b", LightState::Yellow);
        assert_eq!(s.resolved_state(), LightState::Yellow);

        set(&mut s, "c", LightState::Green);
        assert_eq!(s.resolved_state(), LightState::Yellow);
    }

    #[test]
    fn stale_sessions_are_pruned_after_ttl() {
        let mut s = store();
        set(&mut s, "old-red", LightState::Red);
        let pruned_at = Instant::now() + SESSION_TTL + Duration::from_secs(1);

        let expired = s.prune_stale_at(pruned_at, SESSION_TTL);
        assert_eq!(expired, vec!["old-red".to_string()]);
        assert!(s.sessions().is_empty());
        assert_eq!(s.resolved_state(), LightState::Off);
    }

    #[test]
    fn fresh_sessions_survive_the_ttl_sweep() {
        let mut s = store();
        set(&mut s, "busy", LightState::Yellow);
        let expired = s.prune_stale_at(Instant::now(), SESSION_TTL);
        assert!(expired.is_empty());
        assert_eq!(s.resolved_state(), LightState::Yellow);
    }

    #[test]
    fn stale_red_falls_back_to_live_sessions() {
        let mut s = store();
        set(&mut s, "old", LightState::Red);
        set(&mut s, "fresh", LightState::Green);

        // Age only "old": refresh "fresh" at the later timestamp.
        let later = Instant::now() + SESSION_TTL + Duration::from_secs(1);
        s.sessions.get_mut("fresh").unwrap().last_updated = later;
        s.sessions.get_mut("fresh").unwrap().state = LightState::Green;

        let expired = s.prune_stale_at(later, SESSION_TTL);
        assert_eq!(expired, vec!["old".to_string()]);
        assert_eq!(s.resolved_state(), LightState::Green);
    }

    #[test]
    fn pruning_everything_reseeds_the_placeholder() {
        let mut s = store();
        set(&mut s, "gone", LightState::Yellow);
        s.sessions.get_mut("gone").unwrap().last_updated =
            Instant::now() - SESSION_TTL - Duration::from_secs(1);

        let effects = s.prune_stale(SESSION_TTL);
        assert_eq!(effects, vec![AppEffect::Repaint]);
        // The placeholder is re-seeded so the daemon keeps running idle.
        assert_eq!(s.order(), ["agent-1"]);
        assert_eq!(s.resolved_state(), LightState::Green);
    }

    #[test]
    fn prune_stale_keeps_remaining_sessions() {
        let mut s = store();
        set(&mut s, "old", LightState::Red);
        set(&mut s, "new", LightState::Yellow);
        s.sessions.get_mut("old").unwrap().last_updated =
            Instant::now() - SESSION_TTL - Duration::from_secs(1);

        let effects = s.prune_stale(SESSION_TTL);
        assert_eq!(effects, vec![AppEffect::Repaint]);
        assert_eq!(s.order(), ["new"]);
        assert_eq!(s.resolved_state(), LightState::Yellow);
    }
}
