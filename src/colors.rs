/// All possible colors are:
/// - Colors::CAML_WHITE
/// - Colors::CAML_GRAY
/// - Colors::CAML_BLUE
/// - Colors::CAML_BLACK
pub type Color = usize;

pub const CAML_WHITE: Color = 0usize << 8;
pub const CAML_GRAY: Color = 1usize << 8;
pub const CAML_BLUE: Color = 2usize << 8;
pub const CAML_BLACK: Color = 3usize << 8;
