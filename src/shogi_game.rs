use eframe::egui::{CentralPanel, Context, Rect, Vec2, Pos2};
use shogi::{Position, Square, Move, Piece};
use gilrs::{Gilrs, Event, EventType, Button};

use crate::Board;
use crate::piece_button::{self, PIECE_TYPES};

pub struct ShogiGame {
    pos: Position,
    board: Board,
    promotion_flag: bool,
    error_message: String,
    gilrs: Gilrs,
    gamepad_cursor: [i32; 2], // [rank, file]
}

impl ShogiGame {
    pub fn new(pos: Position, board: Board) -> Self {
        let gilrs = Gilrs::new().expect("Failed to initialize gamepad input");

        Self {
            pos,
            board,
            promotion_flag: false,
            error_message: String::new(),
            gilrs,
            gamepad_cursor: [4, 4], // start roughly centered on the board
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
                    self.pos.make_move(m).unwrap_or_else(|err| {
                        self.error_message = format!("Error in make_move: {}", err);
                        Default::default()
                    });
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
                self.pos.make_move(m).unwrap_or_else(|err| {
                    self.error_message = format!("Error in make_move: {}", err);
                    Default::default()
                });
            }
            self.board.reset_activity();
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
        let position_factor = 62.22;
        let (offset_x, offset_y) = (106.5, 56.5);
        let board_size = 560.0;

        let confirm = self.poll_gamepad();
        let [cursor_rank, cursor_file] = self.gamepad_cursor;

        let fill = egui::Color32::from_rgba_unmultiplied(60, 110, 40, 128);
        let stroke = egui::Stroke::new(1.0f32, fill);

        ui.add(egui::Image::new(egui::include_image!("images/boards/painting1.jpg")).fit_to_exact_size(egui::vec2(board_size, board_size)));

        for rank in 0..9 {
            for file in 0..9 {
                let min = Pos2::new(board_size - ((file + 1) as f32 * position_factor) + offset_x, rank as f32 * position_factor + offset_y);
                let rect = Rect::from_min_size(min, Vec2::new(60.0, 60.0));

                if self.board.active == [rank as i32, file as i32] {
                    ui.painter().rect(rect, 0.0, fill, stroke);
                }

                let sq = Square::new(file as u8, rank as u8).unwrap();
                let curr_piece = self.pos.piece_at(sq).clone();
                let button = piece_button::piece_button(curr_piece);

                let clicked = ui.put(rect, button).clicked();
                let gamepad_confirmed = confirm && cursor_rank == rank as i32 && cursor_file == file as i32;

                if clicked || gamepad_confirmed {
                    self.handle_piece_move(rank, file, curr_piece);
                }
            }
        }

        for i in 0..14 {
            let p = PIECE_TYPES[i];
            let count = self.pos.hand(p);

            let (x, y) = match p.color {
                shogi::Color::Black => (board_size + offset_x + 25.0, board_size - 10.0 - ((i % 7) as f32 * position_factor)),
                shogi::Color::White => (25.0, offset_y - 1.0 + (i % 7) as f32 * position_factor),
            };
            let rect = Rect::from_min_size(Pos2::new(x, y), Vec2::new(60.0, 60.0));
            let button = piece_button::piece_button(Some(p));

            if count != 0 {
                if self.board.active_hand == i {
                    ui.painter().rect(rect, 0.0, fill, stroke);
                }
                if ui.put(rect, button).clicked() && p.color == self.pos.side_to_move() {
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
                ui.painter().rect(rect, 0.0, fill, stroke);
            }
        }

        // Highlight the gamepad cursor square
        let min = Pos2::new(board_size - ((cursor_file + 1) as f32 * position_factor) + offset_x, cursor_rank as f32 * position_factor + offset_y);
        let rect = Rect::from_min_size(min, Vec2::new(60.0, 60.0));
        let cursor_stroke = egui::Stroke::new(2.0f32, egui::Color32::from_rgba_unmultiplied(200, 200, 40, 200));
        ui.painter().rect_stroke(rect, 0.0, cursor_stroke);
    }

    fn new_game(&mut self) {
        self.board = Board::new();
        self.pos = Position::new();
        self.pos.set_sfen("lnsgkgsnl/1r5b1/ppppppppp/9/9/9/PPPPPPPPP/1B5R1/LNSGKGSNL b - 1").unwrap();
        self.error_message.clear();
    }

    fn undo_move(&mut self) {
        self.pos.unmake_move().unwrap();
        self.error_message.clear();
    }
}

impl eframe::App for ShogiGame {
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        CentralPanel::default().show(ctx, |ui| {
            egui::Frame::default()
                .inner_margin(egui::Margin { left: 100.0, right: 100.0, top: 50.0, bottom: 50.0 })
                .show(ui, |ui| {
                    self.render_pieces(ui);
                    self.render_grid(ui);

                    ui.add_space(390.0);
                    ui.horizontal(|ui| {
                        if ui.button("New game").clicked() {
                            self.new_game();
                        }
                        if ui.button("Undo move").clicked() {
                            self.undo_move();
                        }
                        if ui.button(format!("Promotion: {}", self.promotion_flag)).clicked() {
                            self.promotion_flag = !self.promotion_flag;
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
        });
    }
}