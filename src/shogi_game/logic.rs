use shogi::{Color, MoveError, Piece, PieceType, Position, Square, Move};

use crate::board::Board;
use crate::piece_button::PIECE_TYPES;
use super::{ShogiGame, TurnState, GameMode, PendingPromotion};

impl ShogiGame {
    fn is_promotable(piece_type: PieceType) -> bool {
        matches!(
            piece_type,
            PieceType::Pawn | PieceType::Lance | PieceType::Knight
                | PieceType::Silver | PieceType::Bishop | PieceType::Rook
        )
    }

    fn in_promotion_zone(color: Color, rank: usize) -> bool {
        match color {
            Color::Black => rank < 3,
            Color::White => rank > 5,
        }
    }

    /// A piece with no legal square left to advance to must promote —
    /// pawn/lance reaching the far rank, knight reaching the far two ranks.
    fn is_forced_promotion(piece_type: PieceType, color: Color, to_rank: usize) -> bool {
        match piece_type {
            PieceType::Pawn | PieceType::Lance => {
                (color == Color::Black && to_rank == 0) || (color == Color::White && to_rank == 8)
            }
            PieceType::Knight => {
                (color == Color::Black && to_rank <= 1) || (color == Color::White && to_rank >= 7)
            }
            _ => false,
        }
    }

    /// Actually applies a move to the position, then advances turn state /
    /// notifies the engine or opponent — shared by direct moves, drops, and
    /// resolved promotion prompts.
    fn commit_move(&mut self, m: Move) {
        let mover = self.pos.side_to_move();
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
            Err(err) => self.resolve_move_error(mover, err),
        }
    }

    /// Handles a rejected make_move: repetition and perpetual check end the
    /// game outright (the move was never applied — position is unchanged).
    /// Anything else (InCheck, Nifu, Uchifuzume, ...) means the UI offered
    /// an illegal move, which shouldn't happen but is reported rather than
    /// silently dropped.
    pub(super) fn resolve_move_error(&mut self, mover: Color, err: MoveError) {
        match err {
            MoveError::Repetition => {
                self.turn_state = TurnState::GameOver;
                self.error_message = "Draw by repetition (sennichite).".into();
            }
            MoveError::PerpetualCheckWin => {
                self.turn_state = TurnState::GameOver;
                self.error_message = format!(
                    "{} wins — opponent forced an illegal perpetual check.",
                    Self::color_name(mover)
                );
            }
            MoveError::PerpetualCheckLose => {
                self.turn_state = TurnState::GameOver;
                self.error_message = format!(
                    "{} wins — {} attempted an illegal perpetual check.",
                    Self::color_name(Self::other(mover)),
                    Self::color_name(mover)
                );
            }
            other => {
                self.error_message = format!("Error in make_move: {}", other);
            }
        }
    }

    fn other(c: Color) -> Color {
        match c {
            Color::Black => Color::White,
            Color::White => Color::Black,
        }
    }

    fn color_name(c: Color) -> &'static str {
        match c {
            Color::Black => "Black",
            Color::White => "White",
        }
    }

    pub(super) fn resolve_promotion(&mut self, promote: bool) {
        if let Some(pending) = self.pending_promotion.take() {
            let m = Move::Normal { from: pending.from, to: pending.to, promote };
            self.commit_move(m);
        }
    }

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
                    let from_rank = active[0] as usize;

                    let eligible = Self::is_promotable(ap.piece_type)
                        && (Self::in_promotion_zone(ap.color, from_rank)
                            || Self::in_promotion_zone(ap.color, rank));

                    if eligible && Self::is_forced_promotion(ap.piece_type, ap.color, rank) {
                        self.commit_move(Move::Normal { from: active_sq, to: to_sq, promote: true });
                    } else if eligible {
                        self.pending_promotion = Some(PendingPromotion { from: active_sq, to: to_sq, piece: ap });
                        self.board.reset_activity();
                        return;
                    } else {
                        self.commit_move(Move::Normal { from: active_sq, to: to_sq, promote: false });
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
            if (self.pos.side_to_move() == Color::Black && active_hand >= 7)
                || (self.pos.side_to_move() == Color::White && active_hand < 7)
            {
                let to_sq = Square::new(file as u8, rank as u8).unwrap();
                let m = Move::Drop { to: to_sq, piece_type: PIECE_TYPES[active_hand].piece_type };
                self.commit_move(m);
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
        self.pending_promotion = None;
        self.turn_state = TurnState::AwaitingLocalInput;
    }

    pub(super) fn undo_move(&mut self) {
        self.pos.unmake_move().unwrap();
        self.error_message.clear();
        self.pending_promotion = None;
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

    pub(super) fn start_analysis(&mut self) {
        let sfen = self.pos.to_sfen();
        let multipv = self.analysis_multipv;
        let think_ms = self.engine_think_ms;
        if let Some(engine) = &mut self.engine {
            engine.start_analysis(&sfen, multipv, think_ms);
            self.analysis_running = true;
            self.analysis_lines.clear();
            self.show_analysis_window = true;
        }
    }
}