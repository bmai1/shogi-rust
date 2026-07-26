use shogi::{Color, Move};
use steamworks::{Client, LobbyId, LobbyKey, LobbyListFilter, LobbyType, SteamId, StringFilter, StringFilterKind};
use steamworks::networking_types::{NetworkingIdentity, SendFlags};
use std::sync::mpsc::{self, Receiver, TryRecvError};

const MOVE_CHANNEL: u32 = 0;
const LOBBY_GAME_TAG: &str = "shogi";

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum LobbyRole {
    Host,  // created the lobby, plays Black
    Guest, // joined the lobby, plays White
}

impl LobbyRole {
    pub fn color(self) -> Color {
        match self {
            LobbyRole::Host => Color::Black,
            LobbyRole::Guest => Color::White,
        }
    }
}

#[derive(Clone)]
pub struct LobbyInfo {
    pub id: LobbyId,
    pub owner: SteamId,
    pub member_count: usize,
}

/// Drives Steam lobby matchmaking and P2P move exchange for `GameMode::OnlinePvP`.
///
/// Lifecycle: create/browse/join a lobby -> wait until both players are present
/// -> hand off `(opponent, role)` to the caller, who constructs `ShogiGame`.
/// After that, call `send_move` / `poll_move` once per frame.
pub struct OnlineController {
    client: Client,
    lobby: Option<LobbyId>,
    role: Option<LobbyRole>,
    opponent: Option<SteamId>,

    lobby_list_rx: Option<Receiver<Vec<LobbyId>>>,
    create_rx: Option<Receiver<Result<LobbyId, String>>>,
    join_rx: Option<Receiver<Result<LobbyId, String>>>,
}

impl OnlineController {
    pub fn new(client: Client) -> Self {
        // Auto-accept incoming P2P sessions; we gate who we actually listen to
        // by only ever polling messages once we know our opponent's SteamId.
        client.networking_messages().session_request_callback(|req| {
            req.accept();
        });

        Self {
            client,
            lobby: None,
            role: None,
            opponent: None,
            lobby_list_rx: None,
            create_rx: None,
            join_rx: None,
        }
    }

    // ---- Matchmaking -----------------------------------------------------

    pub fn request_lobby_list(&mut self) {
        let (tx, rx) = mpsc::channel();
        self.lobby_list_rx = Some(rx);
        self.client
            .matchmaking()
            .set_lobby_list_filter(LobbyListFilter {
                string: Some(vec![StringFilter(
                    LobbyKey::new("game"),
                    LOBBY_GAME_TAG,
                    StringFilterKind::Equal,
                )]),
                ..Default::default()
            })
            .request_lobby_list(move |result| {
                let _ = tx.send(result.unwrap_or_default());
            });
    }

    /// Call once per frame. Returns `Some` exactly once, when a previously
    /// requested lobby list has come back.
    pub fn poll_lobby_list(&mut self) -> Option<Vec<LobbyInfo>> {
        let rx = self.lobby_list_rx.as_ref()?;
        match rx.try_recv() {
            Ok(ids) => {
                self.lobby_list_rx = None;
                let mm = self.client.matchmaking();
                Some(
                    ids.into_iter()
                        .map(|id| LobbyInfo {
                            id,
                            owner: mm.lobby_owner(id),
                            member_count: mm.lobby_member_count(id),
                        })
                        .collect(),
                )
            }
            Err(TryRecvError::Empty) => None,
            Err(TryRecvError::Disconnected) => {
                self.lobby_list_rx = None;
                Some(Vec::new())
            }
        }
    }

    pub fn create_lobby(&mut self) {
        let (tx, rx) = mpsc::channel();
        self.create_rx = Some(rx);
        self.client
            .matchmaking()
            .create_lobby(LobbyType::Public, 2, move |result| {
                let _ = tx.send(result.map_err(|e| e.to_string()));
            });
    }

    pub fn join_lobby(&mut self, id: LobbyId) {
        let (tx, rx) = mpsc::channel();
        self.join_rx = Some(rx);
        self.client.matchmaking().join_lobby(id, move |result| {
            let _ = tx.send(result.map_err(|_| "Failed to join lobby".to_string()));
        });
    }

    #[allow(dead_code)]
    pub fn is_waiting_on_steam(&self) -> bool {
        self.create_rx.is_some() || self.join_rx.is_some()
    }

    /// Call once per frame while `create_lobby`/`join_lobby` is pending.
    /// Resolves the lobby, tags it (if we're the host), and records our role.
    pub fn poll_pending(&mut self) -> Option<Result<LobbyRole, String>> {
        if let Some(rx) = &self.create_rx {
            let outcome = match rx.try_recv() {
                Ok(r) => Some(r),
                Err(TryRecvError::Empty) => None,
                Err(TryRecvError::Disconnected) => Some(Err("Lost connection to Steam".into())),
            };
            if let Some(result) = outcome {
                self.create_rx = None;
                return Some(result.map(|id| {
                    self.client.matchmaking().set_lobby_data(id, "game", LOBBY_GAME_TAG);
                    self.lobby = Some(id);
                    self.role = Some(LobbyRole::Host);
                    LobbyRole::Host
                }));
            }
        }

        if let Some(rx) = &self.join_rx {
            let outcome = match rx.try_recv() {
                Ok(r) => Some(r),
                Err(TryRecvError::Empty) => None,
                Err(TryRecvError::Disconnected) => Some(Err("Lost connection to Steam".into())),
            };
            if let Some(result) = outcome {
                self.join_rx = None;
                return Some(result.map(|id| {
                    self.opponent = Some(self.client.matchmaking().lobby_owner(id));
                    self.lobby = Some(id);
                    self.role = Some(LobbyRole::Guest);
                    LobbyRole::Guest
                }));
            }
        }

        None
    }

    /// Host-only: the opponent's SteamId isn't known until they join the
    /// lobby, so poll this once per frame after `create_lobby` resolves.
    pub fn poll_opponent_joined(&mut self) -> Option<SteamId> {
        if self.opponent.is_some() {
            return self.opponent;
        }
        let lobby = self.lobby?;
        let me = self.client.user().steam_id();
        for member in self.client.matchmaking().lobby_members(lobby) {
            if member != me {
                self.opponent = Some(member);
                return self.opponent;
            }
        }
        None
    }

    pub fn leave_lobby(&mut self) {
        if let Some(lobby) = self.lobby.take() {
            self.client.matchmaking().leave_lobby(lobby);
        }
        self.opponent = None;
        self.role = None;
    }

    // ---- Move exchange -----------------------------------------------------

    /// Sends a move to the opponent. Returns `false` if we don't have an
    /// opponent yet or the send failed (caller should surface an error,
    /// since it means the move never left the local machine).
    pub fn send_move(&self, mv: &Move) -> bool {
        let Some(opponent) = self.opponent else {
            return false;
        };
        let data = mv.to_string();
        let identity = NetworkingIdentity::new_steam_id(opponent);
        self.client
            .networking_messages()
            .send_message_to_user(identity, SendFlags::RELIABLE, data.as_bytes(), MOVE_CHANNEL)
            .is_ok()
    }

    /// Call once per frame while waiting on the opponent. Drains the channel
    /// and returns the first well-formed move found (there should only ever
    /// be one in flight, since we lock local input while waiting).
    pub fn poll_move(&self) -> Option<Move> {
        let messages = self.client.networking_messages();
        for msg in messages.receive_messages_on_channel(MOVE_CHANNEL, 4) {
            if let Ok(text) = std::str::from_utf8(msg.data()) {
                if let Some(mv) = Move::from_sfen(text.trim()) {
                    return Some(mv);
                }
            }
        }
        None
    }
}