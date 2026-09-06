use discord_rich_presence::{activity, DiscordIpc, DiscordIpcClient};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const DISCORD_APP_ID: &str = "1535402147045183600";
/// These names must match the Rich Presence Art Assets in the Discord portal.
const LARGE_ASSET: &str = "kamafeu_logo";
const EDIT_ASSET: &str = "status_edit";
const PLAY_ASSET: &str = "status_play";
const RENDER_ASSET: &str = "status_render";
const PROJECT_URL: &str = "https://github.com/studiopomar/kamafeu";
const RELEASES_URL: &str = "https://github.com/studiopomar/kamafeu/releases";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscordActivityState {
    pub details: String,
    pub state: String,
    pub small_asset: String,
    pub small_text: String,
    pub enabled: bool,
}

impl Default for DiscordActivityState {
    fn default() -> Self {
        Self {
            details: "Novo Projeto".to_string(),
            state: "Editando • 0 notas • 120 BPM".to_string(),
            small_asset: EDIT_ASSET.to_string(),
            small_text: "Editando no Kamafeu Studio".to_string(),
            enabled: true,
        }
    }
}

pub enum RpcMessage {
    Update(DiscordActivityState),
    SetEnabled(bool),
    Shutdown,
}

pub struct DiscordRpcManager {
    tx: Sender<RpcMessage>,
    last_state: Option<DiscordActivityState>,
}

impl DiscordRpcManager {
    pub fn new() -> Self {
        let (tx, rx) = channel::<RpcMessage>();

        thread::spawn(move || {
            rpc_worker_loop(rx);
        });

        Self {
            tx,
            last_state: None,
        }
    }

    pub fn update(&mut self, state: DiscordActivityState) {
        if self.last_state.as_ref() != Some(&state) {
            self.last_state = Some(state.clone());
            let _ = self.tx.send(RpcMessage::Update(state));
        }
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        let _ = self.tx.send(RpcMessage::SetEnabled(enabled));
    }
}

impl Default for DiscordRpcManager {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for DiscordRpcManager {
    fn drop(&mut self) {
        let _ = self.tx.send(RpcMessage::Shutdown);
    }
}

fn rpc_worker_loop(rx: Receiver<RpcMessage>) {
    let mut client: Option<DiscordIpcClient> = None;
    let mut current_state: Option<DiscordActivityState> = None;
    let mut enabled = true;
    let mut activity_needs_update = false;
    let mut last_connect_attempt = Instant::now() - Duration::from_secs(30);
    let start_timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;

    loop {
        // Process channel messages
        let mut got_shutdown = false;
        while let Ok(msg) = rx.try_recv() {
            match msg {
                RpcMessage::Update(new_state) => {
                    enabled = new_state.enabled;
                    current_state = Some(new_state);
                    activity_needs_update = true;
                }
                RpcMessage::SetEnabled(e) => {
                    enabled = e;
                    if !enabled {
                        if let Some(ref mut c) = client {
                            let _ = c.clear_activity();
                            let _ = c.close();
                            client = None;
                        }
                    }
                }
                RpcMessage::Shutdown => {
                    got_shutdown = true;
                }
            }
        }

        if got_shutdown {
            if let Some(ref mut c) = client {
                let _ = c.clear_activity();
                let _ = c.close();
            }
            break;
        }

        if !enabled {
            thread::sleep(Duration::from_millis(1000));
            continue;
        }

        // Try connecting if not connected
        if client.is_none() && last_connect_attempt.elapsed() >= Duration::from_secs(10) {
            last_connect_attempt = Instant::now();
            if let Ok(mut new_client) = DiscordIpcClient::new(DISCORD_APP_ID) {
                if new_client.connect().is_ok() {
                    client = Some(new_client);
                    activity_needs_update = true;
                }
            }
        }

        // Update presence if connected
        if activity_needs_update {
            if let Some(ref mut c) = client {
                if let Some(ref st) = current_state {
                    let activity_builder = activity::Activity::new()
                        .details(&st.details)
                        .state(&st.state)
                        .assets(
                            activity::Assets::new()
                                .large_image(LARGE_ASSET)
                                .large_text("Kamafeu Studio • Sintetizador de voz")
                                .small_image(&st.small_asset)
                                .small_text(&st.small_text),
                        )
                        .timestamps(activity::Timestamps::new().start(start_timestamp))
                        .buttons(vec![
                            activity::Button::new("Ver projeto", PROJECT_URL),
                            activity::Button::new("Baixar Kamafeu", RELEASES_URL),
                        ]);

                    if c.set_activity(activity_builder).is_err() {
                        let _ = c.close();
                        client = None;
                    } else {
                        activity_needs_update = false;
                    }
                }
            }
        }

        thread::sleep(Duration::from_millis(1500));
    }
}

pub fn activity_presentation(
    is_rendering: bool,
    is_playing: bool,
    render_progress: f32,
    project_name: &str,
    part_name: &str,
    voicebank_name: &str,
    note_count: usize,
    selected_count: usize,
    bpm: f64,
    enabled: bool,
) -> DiscordActivityState {
    let project_name = if project_name.trim().is_empty() {
        "Novo Projeto"
    } else {
        project_name
    };
    let voicebank_name = if voicebank_name.trim().is_empty() {
        "Sem cantor"
    } else {
        voicebank_name
    };

    let (state, small_asset, small_text) = if is_rendering {
        (
            format!(
                "Exportando WAV • {:.0}%",
                render_progress.clamp(0.0, 1.0) * 100.0
            ),
            RENDER_ASSET,
            "Renderizando áudio",
        )
    } else if is_playing {
        (
            format!("Tocando • {voicebank_name} • {:.0} BPM", bpm),
            PLAY_ASSET,
            "Reprodução em andamento",
        )
    } else {
        let selection = if selected_count > 0 {
            format!(" • {selected_count} selecionada(s)")
        } else {
            String::new()
        };
        (
            format!(
                "Editando {part_name} • {note_count} notas • {:.0} BPM{selection}",
                bpm
            ),
            EDIT_ASSET,
            "Editando no Kamafeu Studio",
        )
    };

    DiscordActivityState {
        details: truncate_discord_text(project_name, 128),
        state: truncate_discord_text(&state, 128),
        small_asset: small_asset.to_string(),
        small_text: small_text.to_string(),
        enabled,
    }
}

fn truncate_discord_text(text: &str, max_chars: usize) -> String {
    let mut chars = text.chars();
    let clipped: String = chars.by_ref().take(max_chars).collect();
    if chars.next().is_some() && max_chars >= 3 {
        let prefix: String = clipped.chars().take(max_chars - 3).collect();
        format!("{prefix}...")
    } else {
        clipped
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn idle_presence_identifies_the_project_and_active_part() {
        let activity = activity_presentation(
            false,
            false,
            0.0,
            "Canção",
            "Voz principal",
            "Cantor",
            12,
            2,
            128.0,
            true,
        );
        assert_eq!(activity.details, "Canção");
        assert_eq!(activity.small_asset, EDIT_ASSET);
        assert_eq!(
            activity.state,
            "Editando Voz principal • 12 notas • 128 BPM • 2 selecionada(s)"
        );
    }

    #[test]
    fn playback_and_rendering_take_priority_over_editing() {
        let playing = activity_presentation(
            false, true, 0.0, "Canção", "Voz", "Cantor", 1, 0, 120.0, true,
        );
        assert_eq!(playing.small_asset, PLAY_ASSET);
        assert_eq!(playing.state, "Tocando • Cantor • 120 BPM");

        let rendering = activity_presentation(
            true, true, 0.375, "Canção", "Voz", "Cantor", 1, 0, 120.0, true,
        );
        assert_eq!(rendering.small_asset, RENDER_ASSET);
        assert_eq!(rendering.state, "Exportando WAV • 38%");
    }

    #[test]
    fn activity_text_stays_within_discords_field_limit() {
        let text = "á".repeat(200);
        let activity =
            activity_presentation(false, false, 0.0, &text, &text, "Cantor", 1, 0, 120.0, true);
        assert_eq!(activity.details.chars().count(), 128);
        assert_eq!(activity.state.chars().count(), 128);
        assert!(activity.details.ends_with("..."));
    }
}
