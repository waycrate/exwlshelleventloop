use iced::mouse::{Cursor, Interaction};
use iced::widget::canvas;
use iced::widget::canvas::{Cache, Event, Geometry, Path, Text};
use iced::{Color, Task as Command};
use iced::{Length, Point, Rectangle, Renderer, Size, Theme};
use iced_layershell::actions::LayershellCustomActions;
use iced_layershell::application;
use iced_layershell::reexport::wl_keyboard::KeymapFormat;
use iced_layershell::reexport::{Anchor, KeyboardInteractivity};
use iced_layershell::settings::{LayerShellSettings, Settings, VirtualKeyboardSettings};
use std::collections::HashMap;
use std::ffi::CString;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use xkbcommon::xkb;

use std::sync::LazyLock;
const SIMPLE_KEY_WIDTH_DIVISOR: f32 = 19.0;
const BACKSPACE_WIDTH_RATIO: f32 = 1.57;
const TAB_WIDTH_RATIO: f32 = 1.55;
const CAPSLOCK_WIDTH_RATIO: f32 = 2.0;
const ENTER_WIDTH_RATIO: f32 = 1.6;
const LEFT_SHIFT_WIDTH_RATIO: f32 = 2.3;
const RIGHT_SHIFT_WIDTH_RATIO: f32 = 2.35;
const SPACE_WIDTH_RATIO: f32 = 8.0;

static ROWS: LazyLock<[Vec<&str>; 5]> = LazyLock::new(|| {
    [
        vec![
            "~", "1", "2", "3", "4", "5", "6", "7", "8", "9", "0", "-", "=", "⌫", "Num", "/", "*",
        ],
        vec![
            "Tab", "Q", "W", "E", "R", "T", "Y", "U", "I", "O", "P", "{", "}", "\\", "7", "8", "9",
        ],
        vec![
            "CAPS", "A", "S", "D", "F", "G", "H", "J", "K", "L", ";", "\"", "Enter", "4", "5", "6",
        ],
        vec![
            "⇧", "Z", "X", "C", "V", "B", "N", "M", ",", ".", "/", "⇧", "1", "2", "3",
        ],
        vec![
            "Ctrl", "Alt", "Cmd", "Space", "AltGr", "Ctrl", "←", "↑", "→", "0", ".",
        ],
    ]
});

pub fn get_keymap_as_file() -> (File, u32) {
    let context = xkb::Context::new(xkb::CONTEXT_NO_FLAGS);

    let keymap = xkb::Keymap::new_from_names(
        &context,
        "",
        "",
        "us", // if no , it is norwegian
        "",
        None,
        xkb::KEYMAP_COMPILE_NO_FLAGS,
    )
    .expect("xkbcommon keymap panicked!");
    let xkb_state = xkb::State::new(&keymap);
    let keymap = xkb_state
        .get_keymap()
        .get_as_string(xkb::KEYMAP_FORMAT_TEXT_V1);
    let keymap = CString::new(keymap).expect("Keymap should not contain interior nul bytes");
    let keymap = keymap.as_bytes_with_nul();
    let dir = std::env::var_os("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    let mut file = tempfile::tempfile_in(dir).expect("File could not be created!");
    file.write_all(keymap).unwrap();
    file.flush().unwrap();
    (file, keymap.len() as u32)
}

struct KeyCoords {
    position: Point,
    size: Size,
}

#[derive(Default)]
struct KeyboardView {
    draw_cache: Cache,
}

#[derive(Debug, Clone)]
enum Message {
    InputKeyPressed(u32),
}

impl TryInto<LayershellCustomActions> for Message {
    type Error = Self;
    fn try_into(self) -> Result<LayershellCustomActions, Self::Error> {
        let Message::InputKeyPressed(key) = self;
        Ok(LayershellCustomActions::VirtualKeyboardPressed { time: 100, key })
    }
}

impl KeyboardView {
    fn new() -> (Self, Command<Message>) {
        (
            Self {
                ..Default::default()
            },
            Command::none(),
        )
    }
    fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::InputKeyPressed(_) => Command::done(message),
        }
    }

    fn style(&self, theme: &iced::Theme) -> iced::theme::Style {
        use iced::theme::Style;
        Style {
            background_color: Color::TRANSPARENT,
            text_color: theme.palette().text,
        }
    }
    fn view(&self) -> iced::Element<Message> {
        canvas(self).height(Length::Fill).width(Length::Fill).into()
    }

    fn namespace(&self) -> String {
        String::from("Iced - Virtual Keyboard")
    }
}

fn main() -> Result<(), iced_layershell::Error> {
    let (file, keymap_size) = get_keymap_as_file();

    application(
        KeyboardView::namespace,
        KeyboardView::update,
        KeyboardView::view,
    )
    .style(KeyboardView::style)
    .settings(Settings {
        layer_settings: LayerShellSettings {
            size: Some((1200, 400)),
            exclusive_zone: 400,
            anchor: Anchor::Bottom | Anchor::Left | Anchor::Right,
            keyboard_interactivity: KeyboardInteractivity::None,
            ..Default::default()
        },
        virtual_keyboard_support: Some(VirtualKeyboardSettings {
            file,
            keymap_size,
            keymap_format: KeymapFormat::XkbV1,
        }),
        ..Default::default()
    })
    .run_with(KeyboardView::new)
}

type KeyboardState = HashMap<String, KeyCoords>;

fn update_keyboard(
    keyboard: &mut HashMap<String, KeyCoords>,
    keyboard_width: f32,
    frame_height: f32,
) {
    let simple_key_width = keyboard_width / SIMPLE_KEY_WIDTH_DIVISOR;
    let simple_key_height = simple_key_width;
    let half_key_height = simple_key_height / 2.0; // For up and down arrow
    let keyboard_height = simple_key_height * 5.0;
    let keyboard_top_pad = (frame_height - keyboard_height) / 2.0;

    let mut key_y: f32 = keyboard_top_pad + 5.0;
    for (row_index, row) in ROWS.iter().enumerate() {
        let mut key_x = 5.0;

        for (key_index, &label) in row.iter().enumerate() {
            let (width_ratio, key_height) = match (row_index, key_index) {
                (0, 13) => (BACKSPACE_WIDTH_RATIO, simple_key_height),
                (1, 0) => (TAB_WIDTH_RATIO, simple_key_height),
                (2, 0) => (CAPSLOCK_WIDTH_RATIO, simple_key_height),
                (2, 12) => (ENTER_WIDTH_RATIO, simple_key_height),
                (3, 0) => (LEFT_SHIFT_WIDTH_RATIO, simple_key_height),
                (3, 11) => (RIGHT_SHIFT_WIDTH_RATIO, simple_key_height),
                (4, 3) => (SPACE_WIDTH_RATIO, simple_key_height),
                (4, 7) => (1.0, half_key_height),
                _ => (1.0, simple_key_height),
            };

            let key_width = simple_key_width * width_ratio;

            let key_pos = Point::new(key_x, key_y);

            keyboard.insert(
                label.to_string(),
                KeyCoords {
                    position: key_pos,
                    size: Size::new(key_width, key_height),
                },
            );
            key_x += key_width + 5.0;
        }
        key_y += simple_key_height + 5.0;
    }
}

// Implement canvas for Keyboard view
impl canvas::Program<Message> for KeyboardView {
    type State = KeyboardState;

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
            let simple_key_width = keyboard_width / 19.0;
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
            frame.fill(&keyboard, Color::TRANSPARENT);

            let mut key_y: f32 = keyboard_top_pad + 5.0;

            for (row_index, row) in ROWS.iter().enumerate() {
                let mut key_x = 5.0;

                for (key_index, &label) in row.iter().enumerate() {
                    let (width_ratio, key_height) = match (row_index, key_index) {
                        (0, 13) => (BACKSPACE_WIDTH_RATIO, simple_key_height),
                        (1, 0) => (TAB_WIDTH_RATIO, simple_key_height),
                        (2, 0) => (CAPSLOCK_WIDTH_RATIO, simple_key_height),
                        (2, 12) => (ENTER_WIDTH_RATIO, simple_key_height),
                        (3, 0) => (LEFT_SHIFT_WIDTH_RATIO, simple_key_height),
                        (3, 11) => (RIGHT_SHIFT_WIDTH_RATIO, simple_key_height),
                        (4, 3) => (SPACE_WIDTH_RATIO, simple_key_height),
                        (4, 7) => (1.0, half_key_height), // Up Arrow
                        _ => (1.0, simple_key_height),
                    };

                    let key_width = simple_key_width * width_ratio;

                    let key_pos = Point::new(key_x, key_y);
                    let key = Path::rectangle(key_pos, Size::new(key_width, key_height));
                    frame.fill(&key, key_fill_color);
                    frame.fill_text(Text {
                        content: label.to_string(),
                        position: Point::new(key_x + key_width / 3.5, key_y + key_height / 3.0),
                        color: letter_color,
                        shaping: iced::widget::text::Shaping::Advanced,
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
                            shaping: iced::widget::text::Shaping::Advanced,
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

    fn mouse_interaction(
        &self,
        _state: &Self::State,
        _bounds: Rectangle,
        _cursor: Cursor,
    ) -> Interaction {
        Interaction::Pointer
    }

    fn update(
        &self,
        state: &mut Self::State,
        event: &canvas::Event,
        bounds: Rectangle,
        cursor: Cursor,
    ) -> Option<iced::widget::canvas::Action<Message>> {
        update_keyboard(state, bounds.width, bounds.height);
        let Event::Mouse(mouse_event) = event else {
            return None;
        };
        if let iced::mouse::Event::ButtonPressed(iced::mouse::Button::Left) = mouse_event {
            if let Some(click_position) = cursor.position_in(bounds) {
                for (label, key_coords) in state.iter() {
                    // Determine the position of the click
                    let key_position = key_coords.position;
                    let key_size = key_coords.size;

                    if click_position.x >= key_position.x
                        && click_position.x <= key_position.x + key_size.width
                        && click_position.y >= key_position.y
                        && click_position.y <= key_position.y + key_size.height
                    {
                        // Clear the cache
                        self.draw_cache.clear();
                        if let Some(key_code) = get_key_code(label) {
                            return Some(canvas::Action::publish(Message::InputKeyPressed(
                                key_code,
                            )));
                        }
                    }
                }
            }
        }
        None
    }
}

// Map keys
fn get_key_code(label: &str) -> Option<u32> {
    match label {
        "Q" => Some(16),
        "W" => Some(17),
        "E" => Some(18),
        "R" => Some(19),
        "T" => Some(20),
        "Y" => Some(21),
        "U" => Some(22),
        "I" => Some(23),
        "O" => Some(24),
        "P" => Some(25),
        "A" => Some(30),
        "S" => Some(31),
        "D" => Some(32),
        "F" => Some(33),
        "G" => Some(34),
        "H" => Some(35),
        "J" => Some(36),
        "K" => Some(37),
        "L" => Some(38),
        "Z" => Some(44),
        "X" => Some(45),
        "C" => Some(46),
        "V" => Some(47),
        "B" => Some(48),
        "N" => Some(49),
        "M" => Some(50),
        "1" => Some(2),
        "2" => Some(3),
        "3" => Some(4),
        "4" => Some(5),
        "5" => Some(6),
        "6" => Some(7),
        "7" => Some(8),
        "8" => Some(9),
        "9" => Some(10),
        "0" => Some(11),
        "~" => Some(41),
        "-" => Some(12),
        "=" => Some(13),
        "⌫" => Some(14),
        "Tab" => Some(15),
        "Space" => Some(57),
        "[" => Some(26),
        "]" => Some(27),
        "\\" => Some(43),
        "CAPS" => Some(58),
        ";" => Some(39),
        "\"" => Some(40),
        "Enter" => Some(28),
        "," => Some(51),
        "." => Some(52),
        _ => None,
    }
}
