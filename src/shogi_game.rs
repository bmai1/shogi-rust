use eframe::egui::{CentralPanel, Rect, Vec2, Pos2, StrokeKind};
use shogi::{Position, Square, Move, Piece};
use gilrs::{Gilrs, Event, EventType, Button};

use crate::board::Board;
use crate::piece_button::{self, PIECE_TYPES};
use crate::engine::{self, UsiEngine};
use crate::controller::OnlineController;

#[derive(Clone, Copy, PartialEq)]
pub enum GameMode {
    VsEngine,
    OnlinePvP,
    Sandbox,
}

#[allow(dead_code)]
#[derive(Clone, Copy, PartialEq)]
pub enum TurnState {
    AwaitingLocalInput,
    AwaitingOpponent,   // engine thinking, or waiting on network move
    GameOver,
}

pub struct ShogiGame {
    pos: Position,
    board: Board,
    promotion_flag: bool,
    error_message: String,
    gilrs: Gilrs,
    gamepad_cursor: [i32; 2], // [rank, file]
    mode: GameMode,
    turn_state: TurnState,
    engine: Option<UsiEngine>,
    engine_think_ms: i32,
    show_engine_settings: bool,
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
            promotion_flag: false,
            error_message: String::new(),
            gilrs,
            gamepad_cursor: [4, 4],
            mode,
            turn_state,
            engine,
            engine_think_ms: 3000,
            show_engine_settings: false,
            return_to_menu: false,
            net,
            local_color, // Use this to flip White board
        }
    }

    pub fn wants_return_to_menu(&mut self) -> bool {
        std::mem::take(&mut self.return_to_menu)
    }

    /// Sends our move to the opponent and locks local input until their reply arrives
    fn send_local_move(&mut self, mv: &Move) {
        if let Some(net) = &self.net {
            if net.send_move(mv) {
                self.turn_state = TurnState::AwaitingOpponent;
            } else {
                self.error_message = "Failed to send move — opponent not connected yet.".into();
            }
        }
    }

    fn handle_piece_move(&mut self, rank: usize, file: usize, curr_piece: Option<Piece>) {
        let active = self.board.active;
        let active_hand = self.board.active_hand;

        if active != [-1, -1] {
            let active_sq = Square::new(active[1] as u8, active[0] as u8).unwrap();
            let active_piece = self.pos.piece_at(active_sq).clone();

            if let Some(ap) = active_piece {
                let can_move = match curr_piece {
                    None => true,
                    Some(cp) => cp.color != ap.color,
                };

                if can_move {
                    let to_sq = Square::new(file as u8, rank as u8).unwrap();
                    let m = if self.promotion_flag
                        && !piece_button::is_promoted(ap.piece_type)
                        && ((rank < 3 && self.pos.side_to_move() == shogi::Color::Black)
                            || (rank > 5 && self.pos.side_to_move() == shogi::Color::White))
                    {
                        Move::Normal { from: active_sq, to: to_sq, promote: true }
                    } else {
                        Move::Normal { from: active_sq, to: to_sq, promote: false }
                    };

                    self.error_message = format!("{}", m);
                    match self.pos.make_move(m) {
                        Ok(_) => {
                            self.error_message = format!("{}", m);
                            self.check_game_over();
                            if self.turn_state != TurnState::GameOver {
                                match self.mode {
                                    GameMode::VsEngine => self.request_engine_move(),
                                    GameMode::OnlinePvP => self.send_local_move(&m),
                                    GameMode::Sandbox => {}
                                }
                            }
                        }
                        Err(err) => {
                            self.error_message = format!("Error in make_move: {}", err);
                        }
                    }
                }

                match curr_piece {
                    Some(cp) if cp.color == ap.color && active != [rank as i32, file as i32] => {
                        self.board.reset_activity();
                        self.board.set_active(rank as i32, file as i32);
                        let sq = Square::new(file as u8, rank as u8).unwrap();
                        let piece = self.pos.piece_at(sq).clone().unwrap();
                        self.board.set_active_moves(&self.pos, Some(sq), piece);
                    }
                    _ => self.board.reset_activity(),
                }
            }
        } else if let Some(cp) = curr_piece {
            if cp.color == self.pos.side_to_move() {
                self.board.reset_activity();
                self.board.set_active(rank as i32, file as i32);
                let sq = Square::new(file as u8, rank as u8).unwrap();
                let piece = self.pos.piece_at(sq).clone().unwrap();
                self.board.set_active_moves(&self.pos, Some(sq), piece);
            }
        } else if active_hand != usize::MAX {
            if (self.pos.side_to_move() == shogi::Color::Black && active_hand >= 7)
                || (self.pos.side_to_move() == shogi::Color::White && active_hand < 7)
            {
                let to_sq = Square::new(file as u8, rank as u8).unwrap();
                let m = Move::Drop { to: to_sq, piece_type: PIECE_TYPES[active_hand].piece_type };

                self.error_message = format!("{}", m);
                match self.pos.make_move(m) {
                    Ok(_) => {
                        self.error_message = format!("{}", m);
                        self.check_game_over();
                        if self.turn_state != TurnState::GameOver {
                            match self.mode {
                                GameMode::VsEngine => self.request_engine_move(),
                                GameMode::OnlinePvP => self.send_local_move(&m),
                                GameMode::Sandbox => {}
                            }
                        }
                    }
                    Err(err) => {
                        self.error_message = format!("Error in make_move: {}", err);
                    }
                }
            }
            self.board.reset_activity();
        }
    }

    fn request_engine_move(&mut self) {
        if let Some(engine) = &mut self.engine {
            engine.request_move(&self.pos.to_sfen(), self.engine_think_ms);
            self.turn_state = TurnState::AwaitingOpponent;
        }
    }

    // Drains pending gamepad events, moves the on-screen cursor with the D-pad,
    // and returns true for exactly one frame when a "confirm" button was pressed.
    fn poll_gamepad(&mut self) -> bool {
        let mut confirm = false;
        while let Some(Event { event, .. }) = self.gilrs.next_event() {
            match event {
                EventType::ButtonPressed(Button::DPadUp, _) => {
                    self.gamepad_cursor[0] = (self.gamepad_cursor[0] - 1).rem_euclid(9);
                }
                EventType::ButtonPressed(Button::DPadDown, _) => {
                    self.gamepad_cursor[0] = (self.gamepad_cursor[0] + 1).rem_euclid(9);
                }
                EventType::ButtonPressed(Button::DPadLeft, _) => {
                    self.gamepad_cursor[1] = (self.gamepad_cursor[1] + 1).rem_euclid(9);
                }
                EventType::ButtonPressed(Button::DPadRight, _) => {
                    self.gamepad_cursor[1] = (self.gamepad_cursor[1] - 1).rem_euclid(9);
                }
                EventType::ButtonPressed(Button::South, _) => {
                    confirm = true;
                }
                _ => {}
            }
        }
        confirm
    }

    /// Whether the board should be drawn flipped 180° (White's perspective).
    /// VsEngine/Sandbox have no local_color and always render Black-side-down.
    fn is_flipped(&self) -> bool {
        self.local_color == Some(shogi::Color::White)
    }

    /// Maps a logical (rank, file) to the (rank, file) actually used for
    /// screen-position math, honoring the current orientation.
    fn display_coords(&self, rank: usize, file: usize) -> (usize, usize) {
        if self.is_flipped() {
            (8 - rank, 8 - file)
        } else {
            (rank, file)
        }
    }

    fn render_grid(&mut self, ui: &mut egui::Ui) {
        let position_factor = 62.22;
        let (offset_x, offset_y) = (106.5, 56.5);
        let board_size = 560.0;
        let painter = ui.painter();
        let stroke = egui::Stroke::new(1.0f32, egui::Color32::BLACK);

        for label in 0..9 {
            let y = label as f32 * position_factor + offset_y;
            painter.line_segment([Pos2::new(offset_x, y), Pos2::new(offset_x + board_size, y)], stroke);
            painter.text(
                Pos2::new(board_size + offset_x + 10.0, y + offset_y - 25.0),
                egui::Align2::CENTER_CENTER,
                ((b'a' + label as u8) as char).to_string(),
                egui::FontId::default(),
                egui::Color32::GRAY,
            );

            let x = label as f32 * position_factor + offset_x;
            painter.line_segment([Pos2::new(x, offset_y), Pos2::new(x, offset_y + board_size)], stroke);
            painter.text(
                Pos2::new(x + 30.0, offset_y - 10.0),
                egui::Align2::CENTER_CENTER,
                (9 - label).to_string(),
                egui::FontId::default(),
                egui::Color32::GRAY,
            );
        }

        let radius = 3.0;
        let fill = egui::Color32::BLACK;
        painter.circle(Pos2::new(3.0 * position_factor + offset_x, 3.0 * position_factor + offset_y), radius, fill, stroke);
        painter.circle(Pos2::new(6.0 * position_factor + offset_x, 3.0 * position_factor + offset_y), radius, fill, stroke);
        painter.circle(Pos2::new(3.0 * position_factor + offset_x, 6.0 * position_factor + offset_y), radius, fill, stroke);
        painter.circle(Pos2::new(6.0 * position_factor + offset_x, 6.0 * position_factor + offset_y), radius, fill, stroke);

        for rank in 0..9 {
            for file in 0..9 {
                if self.board.active_moves[rank][file] {
                    let center = Pos2::new(
                        rank as f32 * position_factor + offset_x + position_factor / 2.0,
                        file as f32 * position_factor + offset_y + position_factor / 2.0,
                    );
                    let fill = egui::Color32::from_rgba_unmultiplied(60, 110, 40, 128);
                    let stroke = egui::Stroke::new(1.0f32, fill);
                    painter.circle(center, 7.0, fill, stroke);
                }
            }
        }
    }

    fn render_pieces(&mut self, ui: &mut egui::Ui) {
        let input_enabled = self.turn_state == TurnState::AwaitingLocalInput;

        let position_factor = 62.22;
        let (offset_x, offset_y) = (106.5, 56.5);
        let board_size = 560.0;

        let confirm = self.poll_gamepad();
        let [cursor_rank, cursor_file] = self.gamepad_cursor;

        let fill = egui::Color32::from_rgba_unmultiplied(60, 110, 40, 128);
        let stroke = egui::Stroke::new(1.0f32, fill);

        ui.add(egui::Image::new(egui::include_image!("images/boards/kaya1.jpg")).fit_to_exact_size(egui::vec2(board_size, board_size)));

        for rank in 0..9 {
            for file in 0..9 {
                let (draw_rank, draw_file) = self.display_coords(rank, file);
                let min = Pos2::new(
                    board_size - ((draw_file + 1) as f32 * position_factor) + offset_x,
                    draw_rank as f32 * position_factor + offset_y,
                );
                let rect = Rect::from_min_size(min, Vec2::new(60.0, 60.0));

                if self.board.active == [rank as i32, file as i32] {
                    ui.painter().rect(rect, 0.0, fill, stroke, egui::StrokeKind::Outside);
                }

                let sq = Square::new(file as u8, rank as u8).unwrap();
                let curr_piece = self.pos.piece_at(sq).clone();
                let button = piece_button::piece_button(curr_piece);

                let clicked = ui.put(rect, button).clicked();
                let gamepad_confirmed = confirm && cursor_rank == rank as i32 && cursor_file == file as i32;

                if input_enabled && (clicked || gamepad_confirmed) {
                    self.handle_piece_move(rank, file, curr_piece);
                }
            }
        }

        for i in 0..14 {
            let p = PIECE_TYPES[i];
            let count = self.pos.hand(p);

            let is_near_side = match self.local_color {
                Some(local) => p.color == local,
                None => p.color == shogi::Color::Black, // VsEngine/Sandbox: unchanged default
            };

            let (x, y) = if is_near_side {
                (board_size + offset_x + 25.0, board_size - 10.0 - ((i % 7) as f32 * position_factor))
            } else {
                (25.0, offset_y - 1.0 + (i % 7) as f32 * position_factor)
            };

            let rect = Rect::from_min_size(Pos2::new(x, y), Vec2::new(60.0, 60.0));
            let button = piece_button::piece_button(Some(p));

            if count != 0 {
                if self.board.active_hand == i {
                    ui.painter().rect(rect, 0.0, fill, stroke, StrokeKind::Outside);
                }
                if input_enabled && ui.put(rect, button).clicked() && p.color == self.pos.side_to_move() {
                    let tmp = self.board.active_hand;
                    self.board.reset_activity();
                    if tmp != i {
                        self.board.set_active_hand(i);
                        self.board.set_active_moves(&self.pos, None, p);
                    }
                }
            } else {
                ui.put(rect, button);
                let fill = egui::Color32::from_rgba_unmultiplied(23, 23, 23, 128);
                let stroke = egui::Stroke::new(1.0f32, fill);
                ui.painter().rect(rect, 0.0, fill, stroke, StrokeKind::Outside);
            }
        }

        // Highlight the gamepad cursor square
        let (draw_cursor_rank, draw_cursor_file) =
            self.display_coords(cursor_rank as usize, cursor_file as usize);
        let min = Pos2::new(
            board_size - ((draw_cursor_file + 1) as f32 * position_factor) + offset_x,
            draw_cursor_rank as f32 * position_factor + offset_y,
        );
        let rect = Rect::from_min_size(min, Vec2::new(60.0, 60.0));
        let cursor_stroke = egui::Stroke::new(2.0f32, egui::Color32::from_rgba_unmultiplied(200, 200, 40, 200));
        ui.painter().rect_stroke(rect, 0.0, cursor_stroke, StrokeKind::Outside);
    }

    fn new_game(&mut self) {
        self.board = Board::new();
        self.pos = Position::new();
        self.pos.set_sfen("lnsgkgsnl/1r5b1/ppppppppp/9/9/9/PPPPPPPPP/1B5R1/LNSGKGSNL b - 1").unwrap();
        self.error_message.clear();
        self.turn_state = TurnState::AwaitingLocalInput;
    }

    fn undo_move(&mut self) {
        self.pos.unmake_move().unwrap();
        self.error_message.clear();
        self.turn_state = TurnState::AwaitingLocalInput;
    }

    fn has_legal_move(pos: &mut Position) -> bool {
        let side = pos.side_to_move();

        // Normal (board) moves
        for rank in 0..9 {
            for file in 0..9 {
                let sq = Square::new(file, rank).unwrap();
                if let Some(piece) = pos.piece_at(sq).clone() {
                    if piece.color != side {
                        continue;
                    }
                    for to_sq in pos.move_candidates(sq, piece) {
                        for &promote in &[false, true] {
                            let m = Move::Normal { from: sq, to: to_sq, promote };
                            if pos.make_move(m).is_ok() {
                                pos.unmake_move().unwrap();
                                return true;
                            }
                        }
                    }
                }
            }
        }

        // Drop moves
        for i in 0..14 {
            let p = PIECE_TYPES[i];
            if p.color != side || pos.hand(p) == 0 {
                continue;
            }
            for rank in 0..9 {
                for file in 0..9 {
                    let sq = Square::new(file, rank).unwrap();
                    if pos.piece_at(sq).clone().is_some() {
                        continue;
                    }
                    let m = Move::Drop { to: sq, piece_type: p.piece_type };
                    if pos.make_move(m).is_ok() {
                        pos.unmake_move().unwrap();
                        return true;
                    }
                }
            }
        }

        false
    }
    
    fn check_game_over(&mut self) {
        let side = self.pos.side_to_move();
        if !Self::has_legal_move(&mut self.pos) {
            self.turn_state = TurnState::GameOver;
            let winner = match side {
                shogi::Color::Black => "White",
                shogi::Color::White => "Black",
            };
            self.error_message = format!("Checkmate! {} wins.", winner);
        }
    }

    pub fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();

        if self.turn_state == TurnState::AwaitingOpponent {
            if let Some(engine) = &mut self.engine {
                if let Some(mv) = engine.poll_bestmove() {
                    match self.pos.make_move(mv) {
                        Ok(_) => {
                            self.error_message = format!("Engine played: {}", mv);
                            self.check_game_over();
                        }
                        Err(err) => self.error_message = format!("Engine move error: {}", err),
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
                        match self.pos.make_move(mv) {
                            Ok(_) => {
                                self.error_message = format!("Opponent played: {}", mv);
                                self.check_game_over();
                            }
                            Err(err) => self.error_message = format!("Opponent move error: {}", err),
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
                    self.render_pieces(ui);
                    self.render_grid(ui);

                    ui.add_space(390.0);

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
                        if ui.button(format!("Promotion: {}", self.promotion_flag)).clicked() {
                            self.promotion_flag = !self.promotion_flag;
                        }
                    });
                    ui.horizontal(|ui| {
                        if self.engine.is_some() {
                            if ui.button("Engine Settings").clicked() {
                                self.show_engine_settings = true;
                            }
                            let engine_busy = self.turn_state == TurnState::AwaitingOpponent;
                            if ui.add_enabled(!engine_busy, egui::Button::new("Make Engine Move")).clicked() {
                                self.request_engine_move();
                            }
                            if engine_busy {
                                ui.label("(thinking...)");
                            }
                        }
                    });

                    // ui.horizontal(|ui| {
                    //     if ui.button("Print SFEN").clicked() {
                    //         println!("{}", self.pos.to_sfen());
                    //     }
                    //     if ui.button("Castle Presets").clicked() {
                    //         self.new_game();
                    //         let castle_sfen = "lnsgkgsnl/1r5b1/ppppppppp/9/9/9/PPPPPPPPP/1B5R1/LNSGKGSNL b - 1 moves 2h7h 8b3b 5i4h 5a6b 4h3h 6b7b 3h2h 7b8b 3i3h 7a7b 6i5h 4a5b 1g1f";
                    //         self.pos.set_sfen(castle_sfen).expect("Failed to set castle position.");
                    //     }
                    // });

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
                    });
        });
    }
}
