use iced::widget::canvas::{self, Frame, Geometry};
use iced::{Color, Font, Point, Rectangle, Size, Theme, mouse};

use jc_terminal::{RenderableGrid, Rgba};
use super::workspace::Message;

pub const CELL_WIDTH: f32 = 8.4;
pub const CELL_HEIGHT: f32 = 17.0;
pub const FONT_SIZE: f32 = 14.0;

pub const LILEX: Font = Font {
    family: iced::font::Family::Name("Lilex"),
    weight: iced::font::Weight::Normal,
    stretch: iced::font::Stretch::Normal,
    style: iced::font::Style::Normal,
};

pub const LILEX_BOLD: Font = Font {
    family: iced::font::Family::Name("Lilex"),
    weight: iced::font::Weight::Bold,
    stretch: iced::font::Stretch::Normal,
    style: iced::font::Style::Normal,
};

pub const LILEX_ITALIC: Font = Font {
    family: iced::font::Family::Name("Lilex"),
    weight: iced::font::Weight::Normal,
    stretch: iced::font::Stretch::Normal,
    style: iced::font::Style::Italic,
};

#[derive(Default)]
pub struct TerminalCanvasState {
    selecting: bool,
    selection_start: Option<(usize, usize)>,
    selection_end: Option<(usize, usize)>,
}

impl TerminalCanvasState {
    fn selection_range(&self) -> Option<((usize, usize), (usize, usize))> {
        match (self.selection_start, self.selection_end) {
            (Some(s), Some(e)) if s != e => {
                Some(if s <= e { (s, e) } else { (e, s) })
            }
            _ => None,
        }
    }
}

pub struct TerminalProgram {
    pub grid: RenderableGrid,
    pub bg_color: Color,
    pub is_claude: bool,
    pub busy: bool,
}

impl TerminalProgram {
    fn is_in_selection(state: &TerminalCanvasState, row: usize, col: usize) -> bool {
        let Some(((sr, sc), (er, ec))) = state.selection_range() else {
            return false;
        };
        if row < sr || row > er {
            return false;
        }
        if sr == er {
            return col >= sc && col <= ec;
        }
        if row == sr {
            return col >= sc;
        }
        if row == er {
            return col <= ec;
        }
        true
    }

    fn extract_text(&self, state: &TerminalCanvasState) -> String {
        let Some(((sr, sc), (er, ec))) = state.selection_range() else {
            return String::new();
        };
        let mut result = String::new();
        for (ri, line) in self.grid.lines.iter().enumerate() {
            if ri < sr || ri > er {
                continue;
            }
            if ri > sr {
                result.push('\n');
            }
            for (ci, cell) in line.iter().enumerate() {
                let in_sel = if sr == er {
                    ci >= sc && ci <= ec
                } else if ri == sr {
                    ci >= sc
                } else if ri == er {
                    ci <= ec
                } else {
                    true
                };
                if in_sel {
                    result.push(if cell.c == '\0' { ' ' } else { cell.c });
                }
            }
        }
        result
            .lines()
            .map(|l| l.trim_end())
            .collect::<Vec<_>>()
            .join("\n")
            .trim_end()
            .to_string()
    }
}

impl canvas::Program<Message> for TerminalProgram {
    type State = TerminalCanvasState;

    fn update(
        &self,
        state: &mut Self::State,
        event: canvas::Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> (canvas::event::Status, Option<Message>) {
        match event {
            canvas::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                if let Some(pos) = cursor.position_in(bounds) {
                    state.selecting = true;
                    state.selection_start = Some((
                        (pos.y / CELL_HEIGHT) as usize,
                        (pos.x / CELL_WIDTH) as usize,
                    ));
                    state.selection_end = state.selection_start;
                    return (canvas::event::Status::Captured, None);
                }
            }
            canvas::Event::Mouse(mouse::Event::CursorMoved { .. }) if state.selecting => {
                if let Some(pos) = cursor.position_in(bounds) {
                    state.selection_end = Some((
                        (pos.y / CELL_HEIGHT) as usize,
                        (pos.x / CELL_WIDTH) as usize,
                    ));
                    return (canvas::event::Status::Captured, None);
                }
            }
            canvas::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                if state.selecting {
                    state.selecting = false;
                    let text = self.extract_text(state);
                    if !text.is_empty() {
                        return (
                            canvas::event::Status::Captured,
                            Some(Message::TerminalTextSelected(text)),
                        );
                    }
                    return (canvas::event::Status::Captured, None);
                }
            }
            _ => {}
        }
        (canvas::event::Status::Ignored, None)
    }

    fn draw(
        &self,
        state: &Self::State,
        renderer: &iced::Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());

        frame.fill_rectangle(Point::ORIGIN, bounds.size(), self.bg_color);

        // Pass 1: cell backgrounds
        for (ri, line) in self.grid.lines.iter().enumerate() {
            let y = ri as f32 * CELL_HEIGHT;
            if y > bounds.height {
                break;
            }
            for (ci, cell) in line.iter().enumerate() {
                let x = ci as f32 * CELL_WIDTH;
                if x > bounds.width {
                    break;
                }

                let is_cursor = self.grid.cursor.visible
                    && ri == self.grid.cursor.line
                    && ci == self.grid.cursor.col;
                let in_selection = Self::is_in_selection(state, ri, ci);

                let bg = if is_cursor {
                    Color::from_rgba(0.8, 0.8, 0.8, 0.9)
                } else if in_selection {
                    Color::from_rgba(0.2, 0.4, 0.7, 0.5)
                } else {
                    rgba_to_color(cell.bg)
                };

                if !approx_eq(bg, self.bg_color) {
                    frame.fill_rectangle(
                        Point::new(x, y),
                        Size::new(CELL_WIDTH, CELL_HEIGHT),
                        bg,
                    );
                }
            }
        }

        // Pass 2: text (character by character for pixel-perfect alignment)
        for (ri, line) in self.grid.lines.iter().enumerate() {
            let y = ri as f32 * CELL_HEIGHT;
            if y > bounds.height {
                break;
            }
            for (ci, cell) in line.iter().enumerate() {
                let x = ci as f32 * CELL_WIDTH;
                if x > bounds.width {
                    break;
                }

                let ch = if cell.c == '\0' { ' ' } else { cell.c };
                if ch == ' ' {
                    continue;
                }

                let is_cursor = self.grid.cursor.visible
                    && ri == self.grid.cursor.line
                    && ci == self.grid.cursor.col;

                let fg = if is_cursor {
                    rgba_to_color(cell.bg)
                } else {
                    rgba_to_color(cell.fg)
                };

                let font = if cell.bold {
                    LILEX_BOLD
                } else if cell.italic {
                    LILEX_ITALIC
                } else {
                    LILEX
                };

                frame.fill_text(canvas::Text {
                    content: ch.to_string(),
                    position: Point::new(x, y + 1.0),
                    color: fg,
                    size: iced::Pixels(FONT_SIZE),
                    font,
                    ..Default::default()
                });

                if cell.underline {
                    frame.fill_rectangle(
                        Point::new(x, y + CELL_HEIGHT - 2.0),
                        Size::new(CELL_WIDTH, 1.0),
                        fg,
                    );
                }
            }
        }

        if self.is_claude && self.busy {
            frame.fill_text(canvas::Text {
                content: "working...".to_string(),
                position: Point::new(4.0, bounds.height - 16.0),
                color: Color::from_rgb(0.4, 0.8, 0.4),
                size: iced::Pixels(11.0),
                font: LILEX,
                ..Default::default()
            });
        }

        vec![frame.into_geometry()]
    }

    fn mouse_interaction(
        &self,
        state: &Self::State,
        _bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> mouse::Interaction {
        if state.selecting {
            mouse::Interaction::Text
        } else {
            mouse::Interaction::default()
        }
    }
}

fn rgba_to_color(c: Rgba) -> Color {
    Color::from_rgba(c.r, c.g, c.b, c.a)
}

fn approx_eq(a: Color, b: Color) -> bool {
    (a.r - b.r).abs() < 0.01
        && (a.g - b.g).abs() < 0.01
        && (a.b - b.b).abs() < 0.01
        && (a.a - b.a).abs() < 0.01
}
