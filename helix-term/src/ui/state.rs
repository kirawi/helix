use helix_core::{syntax::HighlightEvent, Position};
use helix_view::{graphics::Rect, Document, Theme};

use tui::buffer::Buffer as Surface;

// Represent spans w/ a graph, or vec?
pub struct State {}

// Internally used to generate e.g. indent lines. Not needed, but I think it
// makes things more explicit.
enum RenderEvent {}

impl State {
    /// INFO: This function will iterate over the graphemes of each span and create a set of
    /// characters to insert at the correct location.    
    pub fn indent_guides(&mut self) {
        todo!();
    }

    /// INFO: This function will generate a set of render events. At the end, these events are
    /// processed to actually draw to the screen.
    ///
    /// e.g.
    /// ```
    /// fn main() {
    ///     println!("Hello!");
    /// }
    /// ```  
    /// The events will return events to print the graphemes for the first line, print all the graphemes for the second line,
    /// replace the first grapheme of the previous event with an indent-guide symbol, and print all he grapheme sfor the last line.
    /// At the end, these events are merged and then applied to the Surface.
    fn draw(&mut self) {
        self.indent_guides();
    }
}

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
