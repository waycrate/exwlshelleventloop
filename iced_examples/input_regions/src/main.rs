use iced::widget::{button, row};
use iced::{Color, Element, Length, Task as Command};
use iced_layershell::actions::ActionCallback;
use iced_layershell::application;
use iced_layershell::settings::LayerShellSettings;
use iced_layershell::to_layer_message;

pub fn main() -> Result<(), iced_layershell::Error> {
    application(
        || InputRegionExample::new(),
        InputRegionExample::namespace,
        InputRegionExample::update,
        InputRegionExample::view,
    )
    .style(InputRegionExample::style)
    .layer_settings(LayerShellSettings {
        size: Some((400, 400)),
        ..Default::default()
    })
    .run()
}

#[derive(Copy, Clone)]
struct InputRegionExample(pub bool);

#[to_layer_message]
#[derive(Debug, Clone)]
#[doc = "Some docs"]
enum Message {
    SetRegion,
}

impl InputRegionExample {
    fn new() -> (Self, Command<Message>) {
        (Self(false), Command::none())
    }

    fn namespace() -> String {
        String::from("Custom input regions")
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::SetRegion => {
                self.0 = !self.0;
                let val = self.0;
                Command::done(Message::SetInputRegion(ActionCallback::new(
                    move |region| {
                        if val {
                            // Only the button
                            region.add(0, 0, 400, 70);
                        } else {
                            // Entire Screen
                            region.add(0, 0, 400, 400);
                        }
                    },
                )))
            }
            _ => unreachable!(),
        }
    }

    fn view(&self) -> Element<Message> {
        row![
            button(if self.0 { "Set region" } else { "Reset region" }).on_press(Message::SetRegion)
        ]
        .padding(20)
        .spacing(10)
        .width(Length::Fill)
        .into()
    }

    fn style(&self, theme: &iced::Theme) -> iced::theme::Style {
        use iced::theme::Style;
        Style {
            background_color: Color::from_rgba(0.3, 0.3, 0.3, 0.3),
            text_color: theme.palette().text,
        }
    }
}
