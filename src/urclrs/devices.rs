use strum_macros::EnumString;
use num_derive::FromPrimitive;    

#[derive(Debug, Clone, Copy, EnumString, FromPrimitive)]
#[repr(u8)]
#[allow(dead_code, non_camel_case_types)]
pub enum IOPort {
    // General
    CPUBUS, TEXT, NUMB, SUPPORTED = 5, SPECIAL, PROFILE,
    // Graphics
    X, Y, COLOR, BUFFER, G_SPECIAL = 15,
    // Text
    ASCII, CHAR5, CHAR6, ASCII7, UTF8, UTF16, UTF32, T_SPECIAL = 23,
    // Numbers
    INT, UINT, BIN, HEX, FLOAT, FIXED, N_SPECIAL=31,
    // Storage
    ADDR, BUS, PAGE, S_SPECIAL=39,
    // Miscellaneous
    RNG, NOTE, INSTR, NLEG, WAIT, NADDR, DATA, M_SPECIAL,
    // User defined
    UD1, UD2, UD3, UD4, UD5, UD6, UD7, UD8, UD9, UD10, UD11, UD12, UD13, UD14, UD15, UD16,

    GAMEPAD, AXIS, GAMEPAD_INFO,
    KEY,
    MOUSE_X, MOUSE_Y, MOUSE_DX, MOUSE_DY,
    MOUSE_DWHEEL,
    MOUSE_BUTTONS,
    FILE,
}
