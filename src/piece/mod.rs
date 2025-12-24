pub mod pieces;
pub mod move_validators;
pub mod piece_mover;
pub mod piece_movers;

pub fn as_square_str(square:(usize, usize)) -> String{
    let square_col = (b'a' + square.1 as u8) as char;
    format!("{}{}", square_col, square.0+1)
}

pub fn as_move_str(from:(usize, usize), to:(usize, usize)) -> String{
    let from_col = (b'a' + from.1 as u8) as char;
    let to_col = (b'a' + to.1 as u8) as char;
    format!("{}{}{}{}", from_col, from.0+1, to_col, to.0+1)
}