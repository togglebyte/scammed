use std::fs::read_to_string;
use std::sync::mpsc::{self, Sender};
use std::thread;
use std::time::Duration;

use anathema::component::*;
use anathema::default_widgets::{Canvas, CanvasAttribs, Overflow, Text};
use anathema::geometry::{LocalPos, Pos, Size};
use anathema::prelude::*;
use anathema::state::Hex;

use self::instruction::Instruction;

mod instruction;
mod parse;
pub(crate) mod syntax;

#[derive(State)]
struct Line {
    spans: Value<List<Span>>,
}

impl Line {
    pub fn empty() -> Self {
        Self {
            spans: List::empty(),
        }
    }
}

#[derive(State)]
struct Span {
    text: Value<char>,
    bold: Value<bool>,
    foreground: Value<Hex>,
}

impl Span {
    pub fn new(c: char, foreground: Hex, bold: bool) -> Self {
        Self {
            text: c.into(),
            foreground: foreground.into(),
            bold: bold.into(),
        }
    }

    pub fn empty() -> Self {
        Self {
            text: ' '.into(),
            foreground: Hex::from((255, 255, 255)).into(),
            bold: false.into(),
        }
    }
}

#[derive(State)]
struct Doc {
    doc_height: Value<usize>,
    screen_cursor_x: Value<i32>,
    screen_cursor_y: Value<i32>,
    buf_cursor_x: Value<i32>,
    buf_cursor_y: Value<i32>,
    lines: Value<List<Line>>,
    current_instruction: Value<Option<String>>,
    title: Value<String>,
    waiting: Value<String>,
    show_cursor: Value<bool>,
}

impl Doc {
    pub fn new(title: String) -> Self {
        Self {
            doc_height: 1.into(),
            screen_cursor_x: 0.into(),
            screen_cursor_y: 0.into(),
            buf_cursor_x: 0.into(),
            buf_cursor_y: 0.into(),
            lines: List::from_iter(vec![Line::empty()]),
            current_instruction: None.into(),
            title: title.into(),
            waiting: false.to_string().into(),
            show_cursor: true.into(),
        }
    }
}

struct Editor {
    cursor: Pos,
    cell_attribs: CanvasAttribs,
    foreground: Hex,
    instructions: Vec<Instruction>,
    ack: Sender<()>,
}

impl Editor {
    pub fn new(ack: Sender<()>) -> Self {
        Self {
            cursor: Pos::ZERO,
            cell_attribs: CanvasAttribs::new(),
            foreground: Hex::from((255, 255, 255)),
            instructions: vec![],
            ack,
        }
    }

    fn update_cursor(&mut self, state: &mut Doc, overflow: &mut Overflow, size: Size) {
        // Make sure there are enough lines and spans
        while self.cursor.y as usize >= state.lines.len() {
            state.lines.push_back(Line::empty());
        }

        {
            let mut lines = state.lines.to_mut();
            let line = lines.get_mut(self.cursor.y as usize).unwrap();

            let spans = &mut line.to_mut().spans;
            while self.cursor.x as usize > spans.len() {
                spans.push_back(Span::empty());
            }
        }

        let mut screen_cursor = self.cursor - overflow.offset();

        if screen_cursor.y < 0 {
            overflow.scroll_up_by(-screen_cursor.y);
            screen_cursor.y = 0;
        }

        if screen_cursor.y >= size.height as i32 {
            let offset = screen_cursor.y + 1 - size.height as i32;
            overflow.scroll_down_by(offset);
            screen_cursor.y = size.height as i32 - 1;
        }

        state.screen_cursor_x.set(screen_cursor.x);
        state.screen_cursor_y.set(screen_cursor.y);
        state.buf_cursor_x.set(self.cursor.x);
        state.buf_cursor_y.set(self.cursor.y);
    }

    fn apply_inst(&mut self, inst: Instruction, doc: &mut Doc, mut elements: Elements<'_, '_>) {
        doc.current_instruction.set(Some(format!("{inst:?}")));
        elements.query().by_tag("overflow").first(|el, _| {
            let size = el.size();
            let vp = el.to::<Overflow>();

            match inst {
                Instruction::MoveCursor(x, y) => {
                    self.cursor.x = x as i32;
                    self.cursor.y = y as i32;
                    self.update_cursor(doc, vp, size);
                }
                Instruction::Type(c, bold) => {
                    {
                        let mut lines = doc.lines.to_mut();
                        let line = lines.get_mut(self.cursor.y as usize).unwrap();
                        let mut line = line.to_mut();
                        let spans = line.spans.len();
                        line.spans
                            .insert(self.cursor.x as usize, Span::new(c, self.foreground, bold));
                        self.cursor.x += 1;
                    }

                    self.update_cursor(doc, vp, size);
                }
                Instruction::SetForeground(hex) => self.foreground = hex,
                Instruction::Newline { x } => {
                    self.cursor.x = x;
                    self.cursor.y += 1;
                    self.update_cursor(doc, vp, size);
                }
                Instruction::SetX(x) => {
                    self.cursor.x = x as i32;
                    self.update_cursor(doc, vp, size);
                }
                Instruction::Pause(_) => unreachable!(),
                Instruction::Wait => doc.waiting.set(true.to_string()),
                Instruction::HideCursor => {
                    doc.show_cursor.set(false);
                }
            }
        });
    }
}

impl Component for Editor {
    type Message = Instruction;
    type State = Doc;

    fn on_key(
        &mut self,
        key: KeyEvent,
        state: &mut Self::State,
        mut elements: Elements<'_, '_>,
        _: Context<'_>,
    ) {
        state.waiting.set(false.to_string());
        self.ack.send(());
    }

    fn message(
        &mut self,
        inst: Self::Message,
        state: &mut Self::State,
        mut elements: Elements<'_, '_>,
        _: Context<'_>,
    ) {
        self.apply_inst(inst, state, elements);
    }
}

fn insts(lines: Box<[syntax::Line<'_>]>) -> Vec<Instruction> {
    let mut instructions = parse::Parser::new(lines).instructions();
    instructions.pop();
    instructions.insert(0, Instruction::Pause(1000));
    // instructions.push(Instruction::HideCursor);
    instructions
}

fn main() {
    let mut args = std::env::args().skip(1);
    let path = args.next().unwrap();

    let ext = path.rfind(".").unwrap();
    let ext = &path[ext + 1..];

    let code = read_to_string(&path).unwrap();
    let spans = syntax::highlight(&code, ext);
    let mut instructions = insts(spans);

    let mut doc = Document::new("@main");

    let mut backend = TuiBackend::builder()
        .enable_alt_screen()
        .enable_raw_mode()
        .hide_cursor()
        .finish()
        .unwrap();

    let mut runtime = Runtime::new(doc, backend);

    let title = args.next().unwrap();

    let (tx, rx) = mpsc::channel();
    let cid = runtime
        .register_component(
            "main",
            "components/index.aml",
            Editor::new(tx),
            Doc::new(title),
        )
        .unwrap();
    runtime.register_component("status", "components/status.aml", (), ());
    runtime.register_component("footer", "components/footer.aml", (), ());

    let emitter = runtime.emitter();

    std::thread::spawn(move || {
        for i in instructions {
            if let Instruction::Pause(ms) = i {
                thread::sleep(Duration::from_millis(ms));
                continue;
            }

            if let Instruction::Wait = i {
                emitter.emit(cid, i);
                rx.recv();
                continue;
            }

            use rand::Rng;
            let sleep = rand::thread_rng().gen_range(35..85);
            // let sleep = rand::thread_rng().gen_range(3..5);
            std::thread::sleep(Duration::from_millis(sleep));
            emitter.emit(cid, i);
        }
    });

    let mut runtime = runtime.finish().unwrap();
    runtime.run();
}
