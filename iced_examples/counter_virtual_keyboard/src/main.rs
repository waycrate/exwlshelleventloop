use std::ffi::CString;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

use iced::widget::{button, column, row, text, text_input};
use iced::{Alignment, Command, Element, Length, Theme};
use iced_layershell::actions::LayershellCustomActions;
use iced_layershell::reexport::{Anchor, KeyboardInteractivity};
use iced_layershell::settings::{LayerShellSettings, Settings, VirtualKeyboardSettings};
use iced_layershell::Application;

use iced_layershell::reexport::wl_keyboard::KeymapFormat;
use xkbcommon::xkb;

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

pub fn main() -> Result<(), iced_layershell::Error> {
    let (file, keymap_size) = get_keymap_as_file();
    Counter::run(Settings {
        layer_settings: LayerShellSettings {
            size: Some((0, 400)),
            exclusize_zone: 400,
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
}

struct Counter {
    value: i32,
    text: String,
}

#[derive(Debug, Clone, Copy)]
enum WindowDirection {
    Top,
    Left,
    Right,
    Bottom,
}

#[derive(Debug, Clone)]
enum Message {
    IncrementPressed,
    DecrementPressed,
    TextInput(String),
    Direction(WindowDirection),
    InputTest,
}

impl Application for Counter {
    type Message = Message;
    type Flags = ();
    type Theme = Theme;
    type Executor = iced::executor::Default;

    fn new(_flags: ()) -> (Self, Command<Message>) {
        (
            Self {
                value: 0,
                text: "eee".to_string(),
            },
            Command::none(),
        )
    }

    fn namespace(&self) -> String {
        String::from("Counter - Iced")
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::IncrementPressed => {
                self.value += 1;
                Command::none()
            }
            Message::DecrementPressed => {
                self.value -= 1;
                Command::none()
            }
            Message::TextInput(text) => {
                self.text = text;
                Command::none()
            }
            Message::InputTest => Command::single(
                LayershellCustomActions::VirtualKeyboardPressed { time: 100, key: 16 }.into(),
            ),
            Message::Direction(direction) => match direction {
                WindowDirection::Left => Command::batch(vec![
                    Command::single(
                        LayershellCustomActions::AnchorChange(
                            Anchor::Left | Anchor::Top | Anchor::Bottom,
                        )
                        .into(),
                    ),
                    Command::single(LayershellCustomActions::SizeChange((400, 0)).into()),
                ]),
                WindowDirection::Right => Command::batch(vec![
                    Command::single(
                        LayershellCustomActions::AnchorChange(
                            Anchor::Right | Anchor::Top | Anchor::Bottom,
                        )
                        .into(),
                    ),
                    Command::single(LayershellCustomActions::SizeChange((400, 0)).into()),
                ]),
                WindowDirection::Bottom => Command::batch(vec![
                    Command::single(
                        LayershellCustomActions::AnchorChange(
                            Anchor::Bottom | Anchor::Left | Anchor::Right,
                        )
                        .into(),
                    ),
                    Command::single(LayershellCustomActions::SizeChange((0, 400)).into()),
                ]),
                WindowDirection::Top => Command::batch(vec![
                    Command::single(
                        LayershellCustomActions::AnchorChange(
                            Anchor::Top | Anchor::Left | Anchor::Right,
                        )
                        .into(),
                    ),
                    Command::single(LayershellCustomActions::SizeChange((0, 400)).into()),
                ]),
            },
        }
    }

    fn view(&self) -> Element<Message> {
        let center = column![
            button("Increment").on_press(Message::IncrementPressed),
            text(self.value).size(50),
            button("test_q").on_press(Message::InputTest),
            button("Decrement").on_press(Message::DecrementPressed)
        ]
        .padding(20)
        .align_items(Alignment::Center)
        .width(Length::Fill)
        .height(Length::Fill);
        row![
            button("left")
                .on_press(Message::Direction(WindowDirection::Left))
                .height(Length::Fill),
            column![
                button("top")
                    .on_press(Message::Direction(WindowDirection::Top))
                    .width(Length::Fill),
                center,
                text_input("hello", &self.text)
                    .on_input(Message::TextInput)
                    .padding(10),
                button("bottom")
                    .on_press(Message::Direction(WindowDirection::Bottom))
                    .width(Length::Fill),
            ]
            .width(Length::Fill),
            button("right")
                .on_press(Message::Direction(WindowDirection::Right))
                .height(Length::Fill),
        ]
        .padding(20)
        .spacing(10)
        .align_items(Alignment::Center)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    }
}
