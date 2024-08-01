use anathema::default_widgets::CanvasAttribs;
use anathema::state::Hex;
use syntect::easy::HighlightLines;
use syntect::highlighting::{FontStyle, Style, ThemeSet};
use syntect::parsing::SyntaxSet;
use syntect::util::LinesWithEndings;

#[derive(Debug)]
pub struct Span<'a> {
    pub src: &'a str,
    pub fg: Hex,
    pub bold: bool,
}

impl<'a> Span<'a> {
    pub fn take_space(&self) -> (Option<i32>, &str) {
        let count = self.src.bytes().take_while(|b| *b == b' ').count();
        

        let opt_count = match count {
            0 => None,
            n => Some(n as i32),
        };

        (opt_count, &self.src[count..])
    }
}

impl<'a> From<(Style, &'a str)> for Span<'a> {
    fn from((style, src): (Style, &'a str)) -> Self {
        let bold = style.font_style.contains(FontStyle::BOLD);
        let fg = (style.foreground.r, style.foreground.g, style.foreground.b).into();
        Self { src, fg, bold }
    }
}

#[derive(Debug)]
pub struct Line<'a> {
    pub head: Span<'a>,
    pub tail: Box<[Span<'a>]>,
}

pub fn highlight<'a>(src: &'a str) -> Box<[Line<'a>]> {
    let ps = SyntaxSet::load_defaults_newlines();
    let ts = ThemeSet::load_defaults();
    let theme = ThemeSet::get_theme("themes/custom.stTheme").unwrap();

    // let ts = ThemeSet::load_defaults();
    // let theme = &ts.themes["base16-eighties.dark"];

    let syntax = ps.find_syntax_by_extension("rs").unwrap();
    let mut h = HighlightLines::new(syntax, &theme);

    let mut output = vec![];

    let mut n = 0;
    for line in LinesWithEndings::from(src) {
        let mut head = h
            .highlight_line(line, &ps)
            .unwrap()
            .into_iter()
            .map(Span::from)
            .collect::<Vec<_>>();

        let tail = head.split_off(1);

        let head = head.remove(0);
        output.push(Line {
            tail: tail.into_boxed_slice(),
            head,
        });
    }

    output.into_boxed_slice()
}
