use helix_core::{syntax::HighlightEvent, Position};
use helix_view::{graphics::Rect, Document, Theme};

use tui::buffer::Buffer as Surface;

// Represent spans w/ a graph, or vec?
pub struct State {}

pub fn render_text_highlights<H: Iterator<Item = HighlightEvent>>(
    doc: &Document,
    offset: Position,
    viewport: Rect,
    surface: &mut Surface,
    theme: &Theme,
    highlights: H,
    config: &helix_view::editor::Config,
) {
    let mut state = State::new();
    let mut spans = Vec::new();

    // It's slightly more efficient to produce a full RopeSlice from the Rope, then slice that a bunch
    // of times than it is to always call Rope::slice/get_slice (it will internally always hit RSEnum::Light).
    let text = doc.text().slice(..);
    let text_style = theme.get("ui.text");
    'outer: for event in highlights {
        match event {
            HighlightEvent::HighlightStart(span) => spans.push(span),
            HighlightEvent::HighlightEnd => {
                spans.pop();
            }
            HighlightEvent::Source { start, end } => {
                if text.len_chars() < end {
                    state.trailing_cursor();
                }

                // `unwrap_or_else` part is for off-the-end indices of
                // the rope, to allow cursor highlighting at the end
                // of the rope.
                let text = text.get_slice(start..end).unwrap_or_else(|| " ".into());
                let style = spans
                    .iter()
                    .fold(text_style, |acc, span| acc.patch(theme.highlight(span.0)));
                state.push(text, style);
            }
        }
    }
    state.render();
}
