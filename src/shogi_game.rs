use eframe::egui::{CentralPanel};
use shogi::{Piece, Position, Square};
use gilrs::Gilrs;

use crate::board::Board;
use crate::engine::{self, UsiEngine};
use crate::controller::OnlineController;

mod logic;
mod render;
mod input;

#[derive(Clone, Copy, PartialEq)]
pub enum GameMode {
    VsEngine,
    OnlinePvP,
    Sandbox,
}

#[derive(Clone, Copy, PartialEq)]
pub enum TurnState {
    AwaitingLocalInput,
    AwaitingOpponent,   // engine thinking, or waiting on network move
    GameOver,
}

#[derive(Clone)]
pub(crate) struct PendingPromotion {
    pub from: Square,
    pub to: Square,
    pub piece: Piece, // the moving piece, in its unpromoted form
}

pub struct ShogiGame {
    pos: Position,
    board: Board,
    error_message: String,
    pending_promotion: Option<PendingPromotion>,
    promotion_just_opened: bool,
    gilrs: Gilrs,
    gamepad_cursor: [i32; 2], // [rank, file]
    gamepad_active: bool,
    dpad_repeat: u8,
    stick_repeat: u8,
    mode: GameMode,
    turn_state: TurnState,
    engine: Option<UsiEngine>,
    engine_think_ms: i32,
    show_engine_settings: bool,
    show_analysis_window: bool,
    analysis_lines: Vec<crate::engine::AnalysisLine>,
    analysis_running: bool,
    analysis_multipv: u32,
    return_to_menu: bool,
    net: Option<OnlineController>,
    local_color: Option<shogi::Color>,
}

impl ShogiGame {
    pub fn new(
        pos: Position, 
        board: Board, 
        mode: GameMode,
        net: Option<OnlineController>,
        local_color: Option<shogi::Color>,
    ) -> Self {
        let gilrs = Gilrs::new().expect("Failed to initialize gamepad input");

        let engine = match mode {
            GameMode::VsEngine | GameMode::Sandbox => Some (
                UsiEngine::spawn(&engine::engine_path()).expect("Failed to start YaneuraOu")
            ),
            GameMode::OnlinePvP => None,
        };

        // Black moves first
        let turn_state = match (mode, local_color) {
            (GameMode::OnlinePvP, Some(shogi::Color::White)) => TurnState::AwaitingOpponent,
            _ => TurnState::AwaitingLocalInput,
        };

        Self {
            pos,
            board,
            pending_promotion: None,
            promotion_just_opened: false,
            error_message: String::new(),
            gilrs,
            gamepad_cursor: [4, 4],
            gamepad_active: false,
            dpad_repeat: 0,
            stick_repeat: 0,
            mode,
            turn_state,
            engine,
            engine_think_ms: 3000,
            show_engine_settings: false,
            show_analysis_window: false,
            analysis_lines: Vec::new(),
            analysis_running: false,
            analysis_multipv: 10,
            return_to_menu: false,
            net,
            local_color, // Use this to flip White board
        }
    }

    pub fn wants_return_to_menu(&mut self) -> bool {
        std::mem::take(&mut self.return_to_menu)
    }

    pub fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        let confirm = self.poll_gamepad();

        if self.turn_state == TurnState::AwaitingOpponent {
            if let Some(engine) = &mut self.engine {
                if let Some(mv) = engine.poll_bestmove() {
                    let mover = self.pos.side_to_move();
                    match self.pos.make_move(mv) {
                        Ok(_) => {
                            self.error_message = format!("Engine played: {}", mv);
                            self.check_game_over();
                        }
                        Err(err) => self.resolve_move_error(mover, err),
                    }
                    self.board.reset_activity();
                    if self.turn_state != TurnState::GameOver {
                        self.turn_state = TurnState::AwaitingLocalInput;
                    }
                }
            }

            if self.mode == GameMode::OnlinePvP {
                if let Some(net) = &self.net {
                    if let Some(mv) = net.poll_move() {
                        let mover = self.pos.side_to_move();
                        match self.pos.make_move(mv) {
                            Ok(_) => {
                                self.error_message = format!("Opponent played: {}", mv);
                                self.check_game_over();
                            }
                            Err(err) => self.resolve_move_error(mover, err),
                        }
                        self.board.reset_activity();
                        if self.turn_state != TurnState::GameOver {
                            self.turn_state = TurnState::AwaitingLocalInput;
                        }
                    }
                }
            }
        }

        CentralPanel::default().show(ui, |ui| {
            egui::Frame::default()
                .inner_margin(egui::Margin { left: 100, right: 100, top: 50, bottom: 50 })
                .show(ui, |ui| {
                    self.render_pieces(ui, confirm);
                    self.render_grid(ui);
                    self.render_sprite(ui);

                    if let Some(pending) = self.pending_promotion.clone() {
                        let suppress_input = self.promotion_just_opened;
                        self.render_promotion_prompt(ui, pending, confirm, suppress_input);
                    }
                    self.promotion_just_opened = false;

                    ui.add_space(-50.0); // To fix margin from sprite, otherwise add_space(390.0)

                    ui.horizontal(|ui| {
                        let mode_label = match self.mode {
                            GameMode::VsEngine => "vs Engine",
                            GameMode::OnlinePvP => "Online",
                            GameMode::Sandbox => "Sandbox",
                        };
                        ui.label(format!("Mode: {}", mode_label));
                        if ui.button("Menu").clicked() {
                            self.return_to_menu = true;
                        }
                    });

                    ui.horizontal(|ui| {
                        if self.mode != GameMode::OnlinePvP {
                            if ui.button("New game").clicked() {
                                self.new_game();
                            }
                            if ui.button("Undo move").clicked() {
                                self.undo_move();
                            }
                        }
                    });
                    ui.horizontal(|ui| {
                        if self.engine.is_some() {
                            if ui.button("Engine Settings").clicked() {
                                self.show_engine_settings = true;
                            }
                            let engine_busy = self.turn_state == TurnState::AwaitingOpponent;
                            if ui.add_enabled(!engine_busy && !self.analysis_running, egui::Button::new("Engine Analysis")).clicked() {
                                self.start_analysis();
                            }
                            if ui.add_enabled(!engine_busy, egui::Button::new("Make Engine Move")).clicked() {
                                self.request_engine_move();
                            }
                            if engine_busy {
                                ui.label("(thinking...)");
                            }
                        }
                    });

                    if !self.error_message.is_empty() {
                        ui.label(format!("{}", self.error_message));
                    }

                    ctx.request_repaint(); // needed for gamepad state to update every frame
                });
                egui::Window::new("Engine Settings")
                    .open(&mut self.show_engine_settings)
                    .resizable(false)
                    .collapsible(false)
                    .show(ui, |ui| {
                        ui.add(
                            egui::Slider::new(&mut self.engine_think_ms, 1000..=10000)
                                .step_by(1000.0)
                                .text("Thinking time (ms)")
                        );
                        ui.add(
                            egui::Slider::new(&mut self.analysis_multipv, 1..=15)
                                .text("Analysis lines")
                        );
                    });

                let mut show_analysis = self.show_analysis_window;
                egui::Window::new("Engine Analysis")
                    .open(&mut show_analysis)
                    .resizable(true)
                    .collapsible(false)
                    .default_width(420.0)
                    .show(ui, |ui| {
                        self.render_analysis_contents(ui);
                    });
                self.show_analysis_window = show_analysis;
        });
    }
}
