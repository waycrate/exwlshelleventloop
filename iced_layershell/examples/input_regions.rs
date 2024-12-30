use iced::widget::{button, row, Space};
use iced::{Color, Element, Length, Task as Command, Theme};
use iced_layershell::settings::{LayerShellSettings, Settings};
use iced_layershell::to_layer_message;
use iced_layershell::Application;

pub fn main() -> Result<(), iced_layershell::Error> {
    InputRegionExample::run(Settings {
        layer_settings: LayerShellSettings {
            size: Some((400, 400)),
            ..Default::default()
        },
        ..Default::default()
    })
}

struct InputRegionExample;

#[to_layer_message]
#[derive(Debug, Clone)]
#[doc = "Some docs"]
enum Message {
    SetRegion,
    UnsetRegion,
}

impl Application for InputRegionExample {
    type Message = Message;
    type Flags = ();
    type Theme = Theme;
    type Executor = iced::executor::Default;

    fn new(_flags: ()) -> (Self, Command<Message>) {
        (Self, Command::none())
    }

    fn namespace(&self) -> String {
        String::from("Counter - Iced")
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::SetRegion => Command::done(Message::SetInputRegion(|region| {
                // Only the buttons!
                region.add(0, 0, 400, 70);
            })),
            Message::UnsetRegion => Command::done(Message::SetInputRegion(|region| {
                // Entire window!
                region.add(0, 0, 400, 400);
            })),
            _ => unreachable!(),
        }
    }

    fn view(&self) -> Element<Message> {
        // Create the top row with two buttons
        row![
            button("Set region").on_press(Message::SetRegion),
            Space::with_width(Length::Fill),
            button("Reset region").on_press(Message::UnsetRegion),
        ]
        .padding(20)
        .spacing(10)
        .width(Length::Fill)
        .into()
    }

    fn style(&self, theme: &Self::Theme) -> iced_layershell::Appearance {
        use iced_layershell::Appearance;
        Appearance {
            background_color: Color::from_rgba(0.3, 0.3, 0.3, 0.3),
            text_color: theme.palette().text,
        }
    }
}
