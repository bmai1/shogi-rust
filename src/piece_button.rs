use egui::{Button, Image, ImageSource, include_image, Vec2};
use shogi::{Piece, PieceType, Color};

pub fn piece_image_source(piece: Piece) -> ImageSource<'static> {
    match (piece.piece_type, piece.color) {
        (PieceType::Pawn, Color::Black) => include_image!("images/pieces/0FU.png"),
        (PieceType::Pawn, Color::White) => include_image!("images/pieces/1FU.png"),
        (PieceType::Silver, Color::Black) => include_image!("images/pieces/0GI.png"),
        (PieceType::Silver, Color::White) => include_image!("images/pieces/1GI.png"),
        (PieceType::King, Color::Black) => include_image!("images/pieces/0GY.png"),
        (PieceType::King, Color::White) => include_image!("images/pieces/1OU.png"),
        (PieceType::Rook, Color::Black) => include_image!("images/pieces/0HI.png"),
        (PieceType::Rook, Color::White) => include_image!("images/pieces/1HI.png"),
        (PieceType::Bishop, Color::Black) => include_image!("images/pieces/0KA.png"),
        (PieceType::Bishop, Color::White) => include_image!("images/pieces/1KA.png"),
        (PieceType::Knight, Color::Black) => include_image!("images/pieces/0KE.png"),
        (PieceType::Knight, Color::White) => include_image!("images/pieces/1KE.png"),
        (PieceType::Gold, Color::Black) => include_image!("images/pieces/0KI.png"),
        (PieceType::Gold, Color::White) => include_image!("images/pieces/1KI.png"),
        (PieceType::Lance, Color::Black) => include_image!("images/pieces/0KY.png"),
        (PieceType::Lance, Color::White) => include_image!("images/pieces/1KY.png"),
        (PieceType::ProSilver, Color::Black) => include_image!("images/pieces/0NG.png"),
        (PieceType::ProSilver, Color::White) => include_image!("images/pieces/1NG.png"),
        (PieceType::ProKnight, Color::Black) => include_image!("images/pieces/0NK.png"),
        (PieceType::ProKnight, Color::White) => include_image!("images/pieces/1NK.png"),
        (PieceType::ProLance, Color::Black) => include_image!("images/pieces/0NY.png"),
        (PieceType::ProLance, Color::White) => include_image!("images/pieces/1NY.png"),
        (PieceType::ProRook, Color::Black) => include_image!("images/pieces/0RY.png"),
        (PieceType::ProRook, Color::White) => include_image!("images/pieces/1RY.png"),
        (PieceType::ProPawn, Color::Black) => include_image!("images/pieces/0TO.png"),
        (PieceType::ProPawn, Color::White) => include_image!("images/pieces/1TO.png"),
        (PieceType::ProBishop, Color::Black) => include_image!("images/pieces/0UM.png"),
        (PieceType::ProBishop, Color::White) => include_image!("images/pieces/1UM.png"),
    }
}

pub fn piece_button(piece: Option<Piece>) -> Button<'static> {
    let src = match piece {
        Some(p) => piece_image_source(p),
        None => include_image!("images/pieces/empty.png"),
    };
    let image = Image::new(src).fit_to_exact_size(Vec2::splat(60.0));
    Button::image(image).frame(false).min_size(Vec2::splat(60.0))
}

pub fn is_promoted(piece_type: PieceType) -> bool {
    matches!(
        piece_type,
        PieceType::ProPawn | PieceType::ProKnight | PieceType::ProLance
            | PieceType::ProSilver | PieceType::ProRook | PieceType::ProBishop
    )
}

#[allow(dead_code)]
pub fn promoted_piecetype(piece_type: PieceType) -> PieceType {
    match piece_type {
        PieceType::Silver => PieceType::ProSilver,
        PieceType::Knight => PieceType::ProKnight,
        PieceType::Lance => PieceType::ProLance,
        PieceType::Rook => PieceType::ProRook,
        PieceType::Pawn => PieceType::ProPawn,
        PieceType::Bishop => PieceType::ProBishop,
        _ => PieceType::King,
    }
}

pub static PIECE_TYPES: [Piece; 14] = [
    Piece { piece_type: PieceType::Pawn,   color: Color::White },
    Piece { piece_type: PieceType::Lance,  color: Color::White },
    Piece { piece_type: PieceType::Knight, color: Color::White },
    Piece { piece_type: PieceType::Silver, color: Color::White },
    Piece { piece_type: PieceType::Gold,   color: Color::White },
    Piece { piece_type: PieceType::Bishop, color: Color::White },
    Piece { piece_type: PieceType::Rook,   color: Color::White },
    Piece { piece_type: PieceType::Pawn,   color: Color::Black },
    Piece { piece_type: PieceType::Lance,  color: Color::Black },
    Piece { piece_type: PieceType::Knight, color: Color::Black },
    Piece { piece_type: PieceType::Silver, color: Color::Black },
    Piece { piece_type: PieceType::Gold,   color: Color::Black },
    Piece { piece_type: PieceType::Bishop, color: Color::Black },
    Piece { piece_type: PieceType::Rook,   color: Color::Black },
];