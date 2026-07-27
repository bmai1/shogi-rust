use eframe::egui::{self, Rect, Vec2, Pos2, StrokeKind};
use shogi::Square;

use crate::piece_button::{self, PIECE_TYPES};
use super::{ShogiGame, TurnState};   // `super` reaches back to shogi_game.rs's scope

impl ShogiGame {
    pub(super) fn is_flipped(&self) -> bool {
        self.local_color == Some(shogi::Color::White)
    }

    // Maps a logical (rank, file) to the (rank, file) actually used for
    // screen-position math, honoring the current orientation.
    pub(super) fn display_coords(&self, rank: usize, file: usize) -> (usize, usize) {
        if self.is_flipped() {
            (8 - rank, 8 - file)
        } else {
            (rank, file)
        }
    }

    pub(super) fn render_grid(&mut self, ui: &mut egui::Ui) { 
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
                    let (draw_rank, draw_file) = self.display_coords(rank, file);
                    let center = Pos2::new(
                        board_size - ((draw_file + 1) as f32 * position_factor) + offset_x + position_factor / 2.0,
                        draw_rank as f32 * position_factor + offset_y + position_factor / 2.0,
                    );
                    let fill = egui::Color32::from_rgba_unmultiplied(60, 110, 40, 128);
                    let stroke = egui::Stroke::new(1.0f32, fill);
                    painter.circle(center, 7.0, fill, stroke);
                }
            }
        }
    }
    
    pub(super) fn render_pieces(&mut self, ui: &mut egui::Ui) {
        let input_enabled = self.turn_state == TurnState::AwaitingLocalInput;

        let position_factor = 62.22;
        let (offset_x, offset_y) = (106.5, 56.5);
        let board_size = 560.0;

        let confirm = self.poll_gamepad();
        let [cursor_rank, cursor_file] = self.gamepad_cursor;

        let fill = egui::Color32::from_rgba_unmultiplied(60, 110, 40, 128);
        let stroke = egui::Stroke::new(1.0f32, fill);

        let perspective = self.local_color.unwrap_or(shogi::Color::Black);

        ui.add(egui::Image::new(egui::include_image!("../images/boards/kaya1.jpg")).fit_to_exact_size(egui::vec2(board_size, board_size)));

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
                let button = piece_button::piece_button(curr_piece, perspective);

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
            let button = piece_button::piece_button(Some(p), perspective);

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
        let cursor_stroke = egui::Stroke::new(2.0f32, egui::Color32::from_rgba_unmultiplied(255, 40, 130, 200));
        ui.painter().rect_stroke(rect, 0.0, cursor_stroke, StrokeKind::Outside);
    }
}