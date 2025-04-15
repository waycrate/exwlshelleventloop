use iced_core::{
    Color, Padding, Point, Rectangle, Size, Text, Vector, alignment, input_method, renderer, text,
};

use enumflags2::bitflags;

pub struct Preedit<Renderer>
where
    Renderer: text::Renderer,
{
    position: Point,
    content: Renderer::Paragraph,
    spans: Vec<text::Span<'static, (), Renderer::Font>>,
}

#[bitflags]
#[derive(Debug, Clone, Copy)]
#[repr(u64)]
pub enum ImeState {
    Disabled = 1,
    Allowed = 2,
    Update = 4,
}

impl<Renderer> Preedit<Renderer>
where
    Renderer: text::Renderer,
{
    pub fn new() -> Self {
        Self {
            position: Point::ORIGIN,
            spans: Vec::new(),
            content: Renderer::Paragraph::default(),
        }
    }

    pub fn update(
        &mut self,
        position: Point,
        preedit: &input_method::Preedit,
        background: Color,
        renderer: &Renderer,
    ) {
        self.position = position;

        let spans = match &preedit.selection {
            Some(selection) => {
                vec![
                    text::Span::new(&preedit.content[..selection.start]),
                    text::Span::new(if selection.start == selection.end {
                        "\u{200A}"
                    } else {
                        &preedit.content[selection.start..selection.end]
                    })
                    .color(background),
                    text::Span::new(&preedit.content[selection.end..]),
                ]
            }
            _ => vec![text::Span::new(&preedit.content)],
        };

        if spans != self.spans.as_slice() {
            use text::Paragraph as _;

            self.content = Renderer::Paragraph::with_spans(Text {
                content: &spans,
                bounds: Size::INFINITY,
                size: preedit.text_size.unwrap_or_else(|| renderer.default_size()),
                line_height: text::LineHeight::default(),
                font: renderer.default_font(),
                align_x: text::Alignment::Default,
                align_y: alignment::Vertical::Top,
                shaping: text::Shaping::Advanced,
                wrapping: text::Wrapping::None,
            });

            self.spans.clear();
            self.spans
                .extend(spans.into_iter().map(text::Span::to_static));
        }
    }

    pub fn draw(
        &self,
        renderer: &mut Renderer,
        color: Color,
        background: Color,
        viewport: &Rectangle,
    ) {
        use text::Paragraph as _;

        if self.content.min_width() < 1.0 {
            return;
        }

        let mut bounds = Rectangle::new(
            self.position - Vector::new(0.0, self.content.min_height()),
            self.content.min_bounds(),
        );

        bounds.x = bounds
            .x
            .max(viewport.x)
            .min(viewport.x + viewport.width - bounds.width);

        bounds.y = bounds
            .y
            .max(viewport.y)
            .min(viewport.y + viewport.height - bounds.height);

        renderer.with_layer(bounds, |renderer| {
            renderer.fill_quad(
                renderer::Quad {
                    bounds,
                    ..Default::default()
                },
                background,
            );

            renderer.fill_paragraph(&self.content, bounds.position(), color, bounds);

            const UNDERLINE: f32 = 2.0;

            renderer.fill_quad(
                renderer::Quad {
                    bounds: bounds.shrink(Padding {
                        top: bounds.height - UNDERLINE,
                        ..Default::default()
                    }),
                    ..Default::default()
                },
                color,
            );

            for span_bounds in self.content.span_bounds(1) {
                renderer.fill_quad(
                    renderer::Quad {
                        bounds: span_bounds + (bounds.position() - Point::ORIGIN),
                        ..Default::default()
                    },
                    color,
                );
            }
        });
    }
}
