use eframe::egui::{self, CentralPanel};
use shogi::Position;

use crate::board::Board;
use crate::controller::{LobbyInfo, LobbyRole, OnlineController};
use crate::shogi_game::{ShogiGame, GameMode};

const MENU_SIZE: egui::Vec2 = egui::Vec2::new(780.0, 740.0);
const GAME_SIZE: egui::Vec2 = egui::Vec2::new(1220.0, 740.0);

enum Screen {
    Menu,
    Online(OnlineScreen),
    Game(ShogiGame),
}

/// Sub-states of the online flow, all driven by polling `OnlineController`.
enum OnlineStage {
    Browsing,
    Waiting, // create/join in flight, or (host) waiting for a second player
}

struct OnlineScreen {
    controller: OnlineController,
    stage: OnlineStage,
    lobbies: Vec<LobbyInfo>,
    status: String,
}

pub struct ShogiApp {
    screen: Screen,
    steam: Option<steamworks::Client>,
    menu_error: String,
}

impl ShogiApp {
    pub fn new() -> Self {
        Self {
            screen: Screen::Menu,
            steam: None,
            menu_error: String::new(),
        }
    }

    fn ensure_steam_initialized(&mut self) -> bool {
        if self.steam.is_some() {
            return true;
        }
        match steamworks::Client::init_app(480) {
            Ok(client) => {
                self.steam = Some(client);
                println!("Steam init successful.");
                true
            }
            Err(err) => {
                eprintln!("Steam init failed: {err}");
                false
            }
        }
    }

    fn start_game(&mut self, ui: &mut egui::Ui, mode: GameMode, net: Option<OnlineController>, local_color: Option<shogi::Color>) {
        let board = Board::new();
        let mut pos = Position::new();
        pos.set_sfen("lnsgkgsnl/1r5b1/ppppppppp/9/9/9/PPPPPPPPP/1B5R1/LNSGKGSNL b - 1").unwrap();
        ui.ctx().send_viewport_cmd(egui::ViewportCommand::InnerSize(GAME_SIZE));
        self.screen = Screen::Game(ShogiGame::new(pos, board, mode, net, local_color));
    }

    fn update_online_screen(&mut self, ui: &mut egui::Ui) {
        let mut back_to_menu = false;
        let mut start: Option<(LobbyRole, ())> = None;

        if let Screen::Online(online) = &mut self.screen {
            // Keep ticking every frame while outstanding Steam call
            // or waiting on the opponent otherwise run_callbacks() only
            // fires on user input and async results can sit unclaimed for a while.
            if online.controller.is_waiting_on_steam() || matches!(online.stage, OnlineStage::Waiting) {
                ui.ctx().request_repaint();
            }

            // Resolve create/join once they land.
            if let Some(result) = online.controller.poll_pending() {
                match result {
                    Ok(role) => {
                        online.status = match role {
                            LobbyRole::Host => "Lobby created. Waiting for an opponent...".into(),
                            LobbyRole::Guest => "Joined! Starting game...".into(),
                        };
                        online.stage = OnlineStage::Waiting;
                        if role == LobbyRole::Guest {
                            start = Some((role, ()));
                        }
                    }
                    Err(err) => {
                        online.status = format!("Error: {err}");
                        online.stage = OnlineStage::Browsing;
                    }
                }
            }

            // Host-only: wait for the second lobby member to show up.
            if start.is_none()
                && online.controller.role() == Some(LobbyRole::Host)
                && matches!(online.stage, OnlineStage::Waiting)
                && online.controller.poll_opponent_joined().is_some()
            {
                start = Some((LobbyRole::Host, ()));
            }

            // Lobby list refresh landing.
            if let Some(list) = online.controller.poll_lobby_list() {
                online.lobbies = list;
            }

            CentralPanel::default().show(ui, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(60.0);
                    ui.heading("Online Match");
                    ui.add_space(20.0);

                    if !online.status.is_empty() {
                        ui.label(&online.status);
                        ui.add_space(10.0);
                    }

                    match online.stage {
                        OnlineStage::Waiting => {
                            ui.label("Please wait...");
                        }
                        OnlineStage::Browsing => {
                            if ui.add_sized([200.0, 40.0], egui::Button::new("Host Game")).clicked() {
                                online.controller.create_lobby();
                                online.status = "Creating lobby...".into();
                            }
                            ui.add_space(10.0);
                            if ui.add_sized([200.0, 40.0], egui::Button::new("Refresh Lobby List")).clicked() {
                                online.controller.request_lobby_list();
                                online.status = "Searching for lobbies...".into();
                            }
                            ui.add_space(10.0);

                            let list_width = 400.0;
                            ui.horizontal(|ui| {
                                let margin = ((ui.available_width() - list_width) / 2.0).max(0.0);
                                if online.lobbies.is_empty() {
                                    ui.add_space(margin);
                                }
                                else {
                                    ui.add_space(margin + 100.0);
                                }
                                ui.vertical(|ui| {
                                    ui.set_width(list_width);
                                    egui::ScrollArea::vertical()
                                        .max_height(240.0)
                                        .auto_shrink([false, true])
                                        .show(ui, |ui| {
                                            if online.lobbies.is_empty() {
                                                ui.vertical_centered(|ui| {
                                                    ui.label("No lobbies found yet.");
                                                });
                                            }
                                            for lobby in online.lobbies.clone() {
                                                let host_name = online.controller.display_name(lobby.owner);
                                                ui.horizontal(|ui| {
                                                    ui.label(format!("{}'s game ({}/2 players)", host_name, lobby.member_count));
                                                    if ui.button("Join").clicked() {
                                                        online.controller.join_lobby(lobby.id);
                                                        online.status = "Joining lobby...".into();
                                                    }
                                                });
                                            }
                                        });
                                });
                            });
                        }
                    }

                    ui.add_space(20.0);
                    if ui.button("Back").clicked() {
                        online.controller.leave_lobby();
                        back_to_menu = true;
                    }
                });
            });
        }

        if let Some((role, _)) = start {
            if let Screen::Online(online) = std::mem::replace(&mut self.screen, Screen::Menu) {
                self.start_game(ui, GameMode::OnlinePvP, Some(online.controller), Some(role.color()));
            }
        } else if back_to_menu {
            self.screen = Screen::Menu;
        }
    }
}

impl eframe::App for ShogiApp {
    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        if let Some(client) = &self.steam {
            client.run_callbacks();
        }
        match &mut self.screen {
            Screen::Menu => {
                let mut chosen_mode: Option<GameMode> = None;
                let mut go_online = false;

                egui::CentralPanel::default().show(ui, |ui| {
                    ui.vertical_centered(|ui| {
                        ui.add_space(180.0);
                        ui.heading(egui::RichText::new("Shogi").size(64.0));
                        ui.add_space(50.0);

                        if ui.add_sized([240.0, 50.0], egui::Button::new("Start AI Match")).clicked() {
                            chosen_mode = Some(GameMode::VsEngine);
                        }
                        ui.add_space(15.0);
                        if ui.add_sized([240.0, 50.0], egui::Button::new("Start Online Match")).clicked() {
                             if self.ensure_steam_initialized() {
                                self.menu_error.clear();
                                go_online = true;
                            } else {
                                self.menu_error = String::from(
                                    "Couldn't connect to Steam. Make sure Steam is running and you're logged in, then try again."
                                );
                            }
                        }
                        ui.add_space(15.0);
                        if ui.add_sized([240.0, 50.0], egui::Button::new("Sandbox")).clicked() {
                            chosen_mode = Some(GameMode::Sandbox);
                        }
                        if !self.menu_error.is_empty() {
                            ui.add_space(15.0);
                            ui.colored_label(egui::Color32::from_rgb(200, 60, 60), &self.menu_error);
                        }
                    });
                });

                if let Some(mode) = chosen_mode {
                    self.start_game(ui, mode, None, None);
                } else if go_online {
                    let client = self.steam.as_ref().unwrap().clone();
                    self.screen = Screen::Online(OnlineScreen {
                        controller: OnlineController::new(client),
                        stage: OnlineStage::Browsing,
                        lobbies: Vec::new(),
                        status: String::new(),
                    });
                }
            }
            Screen::Online(_) => {
                self.update_online_screen(ui);
            }
            Screen::Game(game) => {
                game.ui(ui, frame);
                if game.wants_return_to_menu() {
                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::InnerSize(MENU_SIZE));
                    self.screen = Screen::Menu;
                }
            }
        }
    }
}