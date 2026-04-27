use alacritty_terminal::grid::Dimensions;
use alacritty_terminal::index::{Column, Line};
use alacritty_terminal::term::TermMode;
use alacritty_terminal::term::cell::Flags as CellFlags;

use crate::colors::{Palette, Rgba};
use crate::terminal::EventProxy;

#[derive(Debug, Clone)]
pub struct RenderableCell {
    pub c: char,
    pub fg: Rgba,
    pub bg: Rgba,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
}

#[derive(Debug, Clone)]
pub struct CursorState {
    pub line: usize,
    pub col: usize,
    pub visible: bool,
}

#[derive(Debug, Clone)]
pub struct RenderableGrid {
    pub lines: Vec<Vec<RenderableCell>>,
    pub cursor: CursorState,
    pub cols: usize,
    pub rows: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::terminal::{EventProxy, TermDimensions};
    use alacritty_terminal::term::{Config, Term};

    fn make_term(cols: usize, rows: usize) -> Term<EventProxy> {
        let (tx, _rx) = flume::unbounded();
        let proxy = EventProxy::new(tx);
        let dims = TermDimensions { cols, rows };
        Term::new(Config::default(), &dims, proxy)
    }

    #[test]
    fn render_grid_dimensions_match() {
        let term = make_term(80, 24);
        let palette = Palette::default();
        let grid = render_grid(&term, &palette);
        assert_eq!(grid.cols, 80);
        assert_eq!(grid.rows, 24);
        assert_eq!(grid.lines.len(), 24);
        assert_eq!(grid.lines[0].len(), 80);
    }

    #[test]
    fn render_grid_cursor_visible_by_default() {
        let term = make_term(80, 24);
        let palette = Palette::default();
        let grid = render_grid(&term, &palette);
        assert!(grid.cursor.visible);
        assert_eq!(grid.cursor.line, 0);
        assert_eq!(grid.cursor.col, 0);
    }

    #[test]
    fn render_grid_small_terminal() {
        let term = make_term(5, 3);
        let palette = Palette::default();
        let grid = render_grid(&term, &palette);
        assert_eq!(grid.cols, 5);
        assert_eq!(grid.rows, 3);
        assert_eq!(grid.lines.len(), 3);
        for line in &grid.lines {
            assert_eq!(line.len(), 5);
        }
    }

    #[test]
    fn render_grid_cells_have_default_colors() {
        let term = make_term(10, 5);
        let palette = Palette::default();
        let grid = render_grid(&term, &palette);
        let cell = &grid.lines[0][0];
        assert_eq!(cell.fg, palette.foreground);
        assert_eq!(cell.bg, palette.background);
    }

    #[test]
    fn render_grid_empty_cells_are_space() {
        let term = make_term(10, 5);
        let palette = Palette::default();
        let grid = render_grid(&term, &palette);
        assert_eq!(grid.lines[0][0].c, ' ');
    }

    #[test]
    fn renderable_cell_default_flags() {
        let term = make_term(10, 5);
        let palette = Palette::default();
        let grid = render_grid(&term, &palette);
        let cell = &grid.lines[0][0];
        assert!(!cell.bold);
        assert!(!cell.italic);
        assert!(!cell.underline);
    }
}

pub fn render_grid(
    term: &alacritty_terminal::term::Term<EventProxy>,
    palette: &Palette,
) -> RenderableGrid {
    let grid = term.grid();
    let rows = grid.screen_lines();
    let cols = grid.columns();

    let mut lines = Vec::with_capacity(rows);
    for row_idx in 0..rows {
        let mut line = Vec::with_capacity(cols);
        let row = &grid[Line(row_idx as i32)];
        for col_idx in 0..cols {
            let cell = &row[Column(col_idx)];
            let mut fg = palette.resolve_fg(&cell.fg);
            let mut bg = palette.resolve_bg(&cell.bg);

            if cell.flags.contains(CellFlags::INVERSE) {
                std::mem::swap(&mut fg, &mut bg);
            }

            if cell.flags.contains(CellFlags::DIM) {
                fg = Rgba::new(fg.r * 0.66, fg.g * 0.66, fg.b * 0.66, fg.a);
            }

            line.push(RenderableCell {
                c: cell.c,
                fg,
                bg,
                bold: cell.flags.contains(CellFlags::BOLD),
                italic: cell.flags.contains(CellFlags::ITALIC),
                underline: cell.flags.contains(CellFlags::UNDERLINE),
            });
        }
        lines.push(line);
    }

    let cursor_point = grid.cursor.point;
    let cursor_visible = term.mode().contains(TermMode::SHOW_CURSOR);

    RenderableGrid {
        lines,
        cursor: CursorState {
            line: cursor_point.line.0 as usize,
            col: cursor_point.column.0,
            visible: cursor_visible,
        },
        cols,
        rows,
    }
}
