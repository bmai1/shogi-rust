use shogi::{Position, Square, Move, Piece};

use crate::board::Board;
use crate::piece_button::{self, PIECE_TYPES};
use super::{ShogiGame, TurnState, GameMode};

impl ShogiGame {
    // Sends our move to the opponent and locks local input until their reply arrives
    fn send_local_move(&mut self, mv: &Move) {
        if let Some(net) = &self.net {
            if net.send_move(mv) {
                self.turn_state = TurnState::AwaitingOpponent;
            } else {
                self.error_message = "Failed to send move — opponent not connected yet.".into();
            }
        }
    }

    pub(super) fn handle_piece_move(&mut self, rank: usize, file: usize, curr_piece: Option<Piece>) {
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

    pub(super) fn request_engine_move(&mut self) {
        if let Some(engine) = &mut self.engine {
            engine.request_move(&self.pos.to_sfen(), self.engine_think_ms);
            self.turn_state = TurnState::AwaitingOpponent;
        }
    }

    pub(super) fn new_game(&mut self) {
        self.board = Board::new();
        self.pos = Position::new();
        self.pos.set_sfen("lnsgkgsnl/1r5b1/ppppppppp/9/9/9/PPPPPPPPP/1B5R1/LNSGKGSNL b - 1").unwrap();
        self.error_message.clear();
        self.turn_state = TurnState::AwaitingLocalInput;
    }

    pub(super) fn undo_move(&mut self) {
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

    pub(super) fn check_game_over(&mut self) {
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
}