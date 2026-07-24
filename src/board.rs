use shogi::{Position, Piece, Square};

pub struct Board {
    pub active: [i32; 2],
    pub active_hand: usize,
    pub active_moves: [[bool; 9]; 9],
}

impl Board {
    pub fn new() -> Self {
        Self {
            active: [-1, -1],
            active_hand: usize::MAX,
            active_moves: [[false; 9]; 9],
        }
    }

    pub fn set_active(&mut self, rank: i32, file: i32) {
        if self.active == [rank, file] {
            self.active = [-1, -1];
        } else {
            self.active = [rank, file];
        }
    }

    pub fn set_active_hand(&mut self, i: usize) {
        self.active_hand = i;
    }

    pub fn set_active_moves(&mut self, pos: &Position, sq: Option<Square>, p: Piece) {
        self.active_moves = [[false; 9]; 9];
        if sq.is_none() {
            self.drop_candidates(pos, p);
        } else {
            let moves = pos.move_candidates(sq.unwrap(), p);
            for sq in moves {
                let rank = 8 - (sq.index() / 9);
                let file = sq.index() % 9;
                self.active_moves[rank][file] = true;
            }
        }
    }

    pub fn reset_activity(&mut self) {
        self.set_active(-1, -1);
        self.set_active_hand(usize::MAX);
        self.active_moves = [[false; 9]; 9];
    }

    pub fn drop_candidates(&mut self, pos: &Position, p: Piece) {
        if p.piece_type == shogi::PieceType::Pawn {
            let mut pawn_files = [false; 9];
            for rank in 0..9 {
                for file in 0..9 {
                    let sq = Square::new(file, rank).unwrap();
                    if let Some(piece) = pos.piece_at(sq) {
                        if piece.piece_type == shogi::PieceType::Pawn && piece.color == pos.side_to_move() {
                            pawn_files[file as usize] = true;
                        }
                    }
                }
            }
            for rank in 0..9 {
                for file in 0..9 {
                    let sq = Square::new(file, rank).unwrap();
                    if !pawn_files[file as usize] && pos.piece_at(sq).is_none() {
                        let r = 8 - (sq.index() / 9);
                        let f = sq.index() % 9;
                        self.active_moves[r][f] = true;
                    }
                }
            }
        } else {
            for rank in 0..9 {
                for file in 0..9 {
                    let sq = Square::new(file, rank).unwrap();
                    if pos.piece_at(sq).is_none() {
                        let r = 8 - (sq.index() / 9);
                        let f = sq.index() % 9;
                        self.active_moves[r][f] = true;
                    }
                }
            }
        }
    }
}