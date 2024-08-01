use anathema::state::Hex;

#[derive(Debug)]
pub enum Instruction {
    MoveCursor(u16, u16),
    Type(char),
    SetForeground(Hex),
    Newline { x: i32 },
    SetX(i32),
    Pause(u64),
    Wait,
}
