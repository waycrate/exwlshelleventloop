use iced::mouse::Cursor;
use iced::widget::canvas;
use iced::widget::canvas::{Cache, Geometry, Path, Text};
use iced::{Color, Command};
use iced::{Element, Length, Point, Rectangle, Renderer, Size, Theme};
use iced_layershell::reexport::Anchor;
use iced_layershell::settings::{LayerShellSettings, Settings};
use iced_layershell::Application;
#[derive(Default)]

struct KeyboardView {
    draw_cache: Cache,
}
#[derive(Debug, Clone, Copy)]
enum Message {}

impl Application for KeyboardView {
    type Executor = iced::executor::Default;
    type Message = Message;
    type Theme = iced::Theme;
    type Flags = ();

    fn new(_flags: Self::Flags) -> (Self, Command<Message>) {
        (
            Self {
                ..Default::default()
            },
            Command::none(),
        )
    }

    fn update(&mut self, _message: Self::Message) -> Command<Self::Message> {
        // TODO
        Command::none()
    }

    fn view(&self) -> Element<'_, Self::Message, Self::Theme, Renderer> {
        canvas(self).height(Length::Fill).width(Length::Fill).into()
    }

    fn namespace(&self) -> String {
        String::from("Iced - Virtual Keyboard")
    }
}

fn main() -> Result<(), iced_layershell::Error> {
    KeyboardView::run(Settings {
        layer_settings: LayerShellSettings {
            size: Some((1200, 400)),
            exclusive_zone: 400,
            anchor: Anchor::Bottom | Anchor::Left | Anchor::Right,
            ..Default::default()
        },
        ..Default::default()
    })
}

// Implement cavnas for Keyboard view
impl canvas::Program<Message> for KeyboardView {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: Cursor,
    ) -> Vec<Geometry> {
        let letter_color = Color::BLACK;
        let key_fill_color = Color::from_rgb8(0xD1, 0xD1, 0xD1);

        let keyboard = self.draw_cache.draw(renderer, bounds.size(), |frame| {
            let keyboard_width = frame.width();
            let simple_key_width = keyboard_width / 20.0;
            let simple_key_height = simple_key_width;
            let half_key_height = simple_key_height / 2.0; // For up and down arrow
            let keyboard_height = simple_key_height * 5.0;
            let keyboard_top_pad = (frame.height() - keyboard_height) / 2.0;
            let keyboard = Path::rectangle(
                Point {
                    x: 0.0,
                    y: keyboard_top_pad,
                },
                Size {
                    width: keyboard_width,
                    height: keyboard_height,
                },
            );
            frame.fill(&keyboard, Color::from_rgb8(0xFF, 0xFF, 0xFF));

            let mut key_y: f32 = keyboard_top_pad + 5.0;

            let rows = vec![
                vec![
                    "~", "1", "2", "3", "4", "5", "6", "7", "8", "9", "0", "-", "=", "⌫", "Num",
                    "/", "*",
                ],
                vec![
                    "Tab", "Q", "W", "E", "R", "T", "Y", "U", "I", "O", "P", "{", "}", "\\", "7",
                    "8", "9",
                ],
                vec![
                    "CAPS", "A", "S", "D", "F", "G", "H", "J", "K", "L", ";", "\"", "Enter", "4",
                    "5", "6",
                ], // Row 3
                vec![
                    "⇧", "Z", "X", "C", "V", "B", "N", "M", ",", ".", "/", "⇧", "1", "2", "3",
                ],
                vec![
                    "Ctrl", "Alt", "Cmd", "Space", "AltGr", "Ctrl", "", "←", "↑", "→", "0", ".",
                ],
            ];

            for (row_index, row) in rows.iter().enumerate() {
                let mut key_x = 5.0;

                for (key_index, &label) in row.iter().enumerate() {
                    let (width_ratio, key_height) = match (row_index, key_index) {
                        (0, 13) => (1.57, simple_key_height), // Backspace
                        (1, 0) => (1.55, simple_key_height),  // Tab
                        (2, 0) => (2.0, simple_key_height),   // CapsLock
                        (2, 12) => (1.6, simple_key_height),  // Enter
                        (3, 0) => (2.3, simple_key_height),   // Left Shift
                        (3, 11) => (2.35, simple_key_height), // Right Shift
                        (4, 3) => (6.9, simple_key_height),   // Space
                        (4, 0) => (1.0, simple_key_height),   // Left Ctrl
                        (4, 1) => (1.0, simple_key_height),   // Left Alt
                        (4, 4) => (1.0, simple_key_height),   // Alt
                        (4, 5) => (1.0, simple_key_height),   // Right Ctrl
                        (4, 6) => (1.0, simple_key_height),   // Left Arrow
                        (4, 7) => (1.0, half_key_height),     // Up Arrow
                        (4, 8) => (1.0, simple_key_height),   // Right Arrow
                        _ => (1.0, simple_key_height),        // Default width ratio
                    };

                    let key_width = simple_key_width * width_ratio;

                    let key_pos = Point::new(key_x, key_y);
                    let key = Path::rectangle(key_pos, Size::new(key_width, key_height));
                    frame.fill(&key, key_fill_color);
                    frame.fill_text(Text {
                        content: label.to_string(),
                        position: Point::new(key_x + key_width / 3.5, key_y + key_height / 3.0),
                        color: letter_color,
                        ..Text::default()
                    });

                    key_x += key_width + 5.0;

                    if row_index == 4 && key_index == 7 {
                        let down_arrow_pos =
                            Point::new(key_x - key_width - 5.0, key_y + half_key_height + 5.0);
                        let down_arrow =
                            Path::rectangle(down_arrow_pos, Size::new(key_width, half_key_height));
                        frame.fill(&down_arrow, key_fill_color);
                        frame.fill_text(Text {
                            content: "↓".to_string(),
                            position: Point::new(
                                down_arrow_pos.x + key_width / 3.5,
                                down_arrow_pos.y + half_key_height / 3.0,
                            ),
                            color: letter_color,
                            ..Text::default()
                        });
                    }
                }

                key_y += simple_key_height + 5.0;
            }
        });
        vec![keyboard]
    }
}
