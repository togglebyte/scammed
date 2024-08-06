use std::cmp::Ordering;

use anathema::state::Hex;

use crate::instruction::Instruction;
use crate::syntax::{Span, Line};

pub struct Parser<'a> {
    lines: Box<[Line<'a>]>,
    instructions: Vec<Instruction>,
    foreground: Hex,
}

impl<'a> Parser<'a> {
    pub fn new(lines: Box<[Line<'a>]>) -> Self {
        Self {
            lines,
            instructions: vec![],
            foreground: Hex::BLACK,
        }
    }

    pub fn instructions(mut self) -> Vec<Instruction> {
        let lines = std::mem::take(&mut self.lines);

        for line in &*lines {
            let mut line_start = 0;

            if line.head.src.starts_with("//") {
                if line.tail[0].src.contains("[WAIT]") {
                    self.instructions.push(Instruction::Wait);
                    continue;
                }
            }

            let (count, src, bold) = line.head.take_space();
            if let Some(x) = count {
                self.instructions.push(Instruction::SetX(x));
                line_start = x;
            } else {
                self.instructions.push(Instruction::SetX(0));
            }

            self.set_foreground(&line.head);
            self.push_chars(src, bold, line_start);

            for span in &*line.tail {
                self.set_foreground(span);
                self.push_chars(span.src, span.bold, line_start);
            }
        }

        self.instructions
    }

    fn set_foreground(&mut self, span: &Span) {
        if span.fg != self.foreground {
            self.instructions.push(Instruction::SetForeground(span.fg));
            self.foreground = span.fg;
        }
    }

    fn push_chars(&mut self, src: &str, bold: bool, line_start: i32) {
        for c in src.chars() {
            match c {
                '\n' => self.instructions.push(Instruction::Newline { x: line_start }),
                c => self.instructions.push(Instruction::Type(c, bold)),
            }
        }
    }
}
