use eframe::egui::{self, CentralPanel, Context};
use shogi::Position;

use crate::board::Board;
use crate::shogi_game::{ShogiGame, GameMode};

enum Screen {
    Menu,
    Game(ShogiGame),
}

pub struct ShogiApp {
    screen: Screen,
}

impl ShogiApp {
    pub fn new() -> Self {
        Self { screen: Screen::Menu }
    }

    fn start_game(&mut self, mode: GameMode) {
        let board = Board::new();
        let mut pos = Position::new();
        pos.set_sfen("lnsgkgsnl/1r5b1/ppppppppp/9/9/9/PPPPPPPPP/1B5R1/LNSGKGSNL b - 1").unwrap();
        self.screen = Screen::Game(ShogiGame::new(pos, board, mode));
    }
}

impl eframe::App for ShogiApp {
    fn update(&mut self, ctx: &Context, frame: &mut eframe::Frame) {
        match &mut self.screen {
            Screen::Menu => {
                let mut chosen_mode: Option<GameMode> = None;

                CentralPanel::default().show(ctx, |ui| {
                    ui.vertical_centered(|ui| {
                        ui.add_space(180.0);
                        ui.heading(egui::RichText::new("Shogi").size(64.0));
                        ui.add_space(50.0);

                        if ui.add_sized([240.0, 50.0], egui::Button::new("Start AI Match")).clicked() {
                            chosen_mode = Some(GameMode::VsEngine);
                        }
                        ui.add_space(15.0);
                        if ui.add_sized([240.0, 50.0], egui::Button::new("Start Online Match")).clicked() {
                            chosen_mode = Some(GameMode::OnlinePvP);
                        }
                        ui.add_space(15.0);
                        if ui.add_sized([240.0, 50.0], egui::Button::new("Sandbox")).clicked() {
                            chosen_mode = Some(GameMode::Sandbox);
                        }
                    });
                });

                if let Some(mode) = chosen_mode {
                    self.start_game(mode);
                }
            }
            Screen::Game(game) => {
                game.update(ctx, frame);
                if game.wants_return_to_menu() {
                    self.screen = Screen::Menu;
                }
            }
        }
    }
}