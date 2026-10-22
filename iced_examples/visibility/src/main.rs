use std::{
    io::{self, Write},
    thread,
};

use futures::SinkExt;
use iced::widget::{button, column, container, row, text, text_input};
use iced::{
    Border, Center, Color, Element, Event, Fill, Length, Subscription, Task, Theme, event, keyboard,
};
use iced_exwlshell::actions::ActionCallback;
use iced_exwlshell::layershell::application;
use iced_exwlshell::reexport::{Anchor, BlurOption, KeyboardInteractivity, Layer, LayerSize};
use iced_exwlshell::settings::LayerShellSettings;
use iced_exwlshell::to_layer_message;

const WIDTH: i32 = 560;
const HEIGHT: i32 = 420;

#[derive(Debug, Default)]
struct State {
    note: String,
    cursor_inside: bool,
}

#[to_layer_message]
#[derive(Debug, Clone)]
enum Message {
    Exit,
    NoteChanged(String),
    ClearNote,
    Visibility(bool),
    IcedEvent(Event),
}

fn update(state: &mut State, message: Message) -> Task<Message> {
    match message {
        Message::Exit => iced::exit(),
        Message::NoteChanged(note) => {
            state.note = note;
            Task::none()
        }
        Message::ClearNote => {
            state.note.clear();
            Task::none()
        }
        Message::IcedEvent(Event::Window(iced::window::Event::Opened { .. })) => {
            Task::done(Message::SetInputRegion(input_region(true)))
        }
        Message::IcedEvent(Event::Mouse(iced::mouse::Event::CursorEntered)) => {
            state.cursor_inside = true;
            Task::none()
        }
        Message::IcedEvent(Event::Mouse(iced::mouse::Event::CursorLeft)) => {
            state.cursor_inside = false;
            Task::none()
        }
        Message::IcedEvent(_) => Task::none(),
        Message::Visibility(visible) => {
            let layer = if visible {
                Layer::Top
            } else {
                Layer::Background
            };
            let keyboard = if visible {
                KeyboardInteractivity::OnDemand
            } else {
                KeyboardInteractivity::None
            };

            Task::batch([
                Task::done(Message::LayerChange(layer)),
                Task::done(Message::KeyboardInteractivityChange(keyboard)),
                Task::done(Message::SetInputRegion(input_region(visible))),
            ])
        }
        _ => Task::none(),
    }
}

fn subscription(_: &State) -> Subscription<Message> {
    let keyboard = keyboard::listen().filter_map(|event| {
        let keyboard::Event::KeyPressed { key, modifiers, .. } = event else {
            return None;
        };

        if !modifiers.command() {
            return None;
        }

        match key.as_ref() {
            keyboard::Key::Character("q" | "Q") => Some(Message::Exit),
            keyboard::Key::Character("l" | "L") => Some(Message::ClearNote),
            _ => None,
        }
    });

    Subscription::batch([
        keyboard,
        event::listen().map(Message::IcedEvent),
        terminal_visibility(),
    ])
}

fn input_region(visible: bool) -> ActionCallback {
    ActionCallback::new(move |region| {
        if visible {
            let radius = 22_i32.min(WIDTH / 2).min(HEIGHT / 2);
            for y in 0..HEIGHT {
                let inset = rounded_corner_inset(y, WIDTH, HEIGHT, radius);
                let span = WIDTH - 2 * inset;
                if span > 0 {
                    region.add(inset, y, span, 1);
                }
            }
        }
    })
}

fn terminal_visibility() -> Subscription<Message> {
    Subscription::run(|| {
        iced::stream::channel(
            10,
            |mut output: futures::channel::mpsc::Sender<Message>| async move {
                thread::spawn(move || {
                    let stdin = io::stdin();
                    let mut line = String::new();

                    loop {
                        print!("show window with s, hide window with h. type s/h: ");
                        if let Err(error) = io::stdout().flush() {
                            eprintln!("could not flush the visibility prompt: {error}");
                            break;
                        }

                        line.clear();
                        match stdin.read_line(&mut line) {
                            Ok(0) => break,
                            Ok(_) => {
                                let visible = match line.trim() {
                                    "s" => true,
                                    "h" => false,
                                    _ => {
                                        println!("Please type s or h.");
                                        continue;
                                    }
                                };

                                if futures::executor::block_on(
                                    output.send(Message::Visibility(visible)),
                                )
                                .is_err()
                                {
                                    break;
                                }
                            }
                            Err(error) => {
                                eprintln!("could not read the visibility command: {error}");
                                break;
                            }
                        }
                    }
                });

                futures::future::pending::<()>().await;
            },
        )
    })
}

fn shortcut(key: &'static str, action: &'static str) -> Element<'static, Message> {
    container(
        row![
            text(key).size(13).color(Color::WHITE),
            text(action)
                .size(12)
                .color(Color::from_rgb(0.68, 0.75, 0.85)),
        ]
        .spacing(10)
        .align_y(Center),
    )
    .width(Fill)
    .padding([10, 12])
    .style(|_| {
        container::Style::default()
            .background(Color::from_rgba(0.08, 0.12, 0.2, 0.3))
            .border(Border {
                color: Color::from_rgba(0.72, 0.82, 1.0, 0.18),
                width: 1.0,
                radius: 12.0.into(),
            })
    })
    .into()
}

fn view(state: &State) -> Element<'_, Message> {
    let cursor_status = container(
        row![
            text("●").size(10).color(if state.cursor_inside {
                Color::from_rgb(0.44, 0.94, 0.75)
            } else {
                Color::from_rgb(1.0, 0.64, 0.42)
            }),
            text(if state.cursor_inside {
                "CURSOR INSIDE"
            } else {
                "CURSOR OUTSIDE"
            })
            .size(11)
            .color(Color::WHITE),
        ]
        .spacing(7)
        .align_y(Center),
    )
    .padding([7, 11])
    .style(|_| {
        container::Style::default()
            .background(Color::from_rgba(0.08, 0.12, 0.2, 0.34))
            .border(Border {
                color: Color::from_rgba(0.72, 0.82, 1.0, 0.18),
                width: 1.0,
                radius: 20.0.into(),
            })
    });

    let note_input = column![
        text("QUICK NOTE")
            .size(11)
            .color(Color::from_rgb(0.67, 0.78, 0.98)),
        container(
            text_input("Type something...", &state.note)
                .on_input(Message::NoteChanged)
                .width(Fill)
                .padding([10, 13])
                .size(14)
                .style(|_, status| {
                    let border_color =
                        if matches!(status, iced::widget::text_input::Status::Focused { .. }) {
                            Color::from_rgba(0.55, 0.74, 1.0, 0.72)
                        } else {
                            Color::from_rgba(0.72, 0.82, 1.0, 0.2)
                        };

                    iced::widget::text_input::Style {
                        background: Color::from_rgba(0.08, 0.12, 0.2, 0.3).into(),
                        border: Border {
                            color: border_color,
                            width: 1.0,
                            radius: 12.0.into(),
                        },
                        placeholder: Color::from_rgb(0.55, 0.62, 0.73),
                        value: Color::WHITE,
                        icon: Color::WHITE,
                        selection: Color::from_rgb(0.28, 0.48, 0.78),
                    }
                }),
        )
        .height(Length::Fixed(48.0))
        .center_y(Fill),
    ]
    .spacing(8);

    let shortcuts = container(
        column![
            text("HOW TO USE KEYBOARD SHORTCUTS")
                .size(11)
                .color(Color::from_rgb(0.67, 0.78, 0.98)),
            text("Click this panel to focus it, then press:")
                .size(12)
                .color(Color::from_rgb(0.86, 0.9, 0.97)),
            row![
                shortcut("CTRL + Q", "Exit demo"),
                shortcut("CTRL + L", "Clear Quick Note"),
            ]
            .spacing(10),
        ]
        .spacing(8),
    )
    .width(Fill)
    .padding(13)
    .style(|_| {
        container::Style::default()
            .background(Color::from_rgba(0.12, 0.2, 0.34, 0.28))
            .border(Border {
                color: Color::from_rgba(0.55, 0.72, 1.0, 0.3),
                width: 1.0,
                radius: 14.0.into(),
            })
    });

    container(
        column![
            row![
                column![
                    text("LAYER STUDIO")
                        .size(11)
                        .color(Color::from_rgb(0.67, 0.78, 0.98)),
                    text("A calmer desktop.").size(27).color(Color::WHITE),
                ]
                .spacing(5),
                cursor_status,
                button(text("Exit").size(11))
                    .on_press(Message::Exit)
                    .padding([7, 11])
                    .style(|_, _| iced::widget::button::Style {
                        background: Some(Color::from_rgba(0.08, 0.12, 0.2, 0.34).into()),
                        text_color: Color::WHITE,
                        border: Border {
                            color: Color::from_rgba(0.72, 0.82, 1.0, 0.18),
                            width: 1.0,
                            radius: 20.0.into(),
                        },
                        ..iced::widget::button::Style::default()
                    }),
            ]
            .align_y(Center)
            .spacing(12),
            text("Wayland layer shell, rendered with Iced.")
                .size(14)
                .color(Color::from_rgb(0.74, 0.8, 0.89)),
            text("Move across a rounded corner to see the cursor leave the surface.")
                .size(12)
                .color(Color::from_rgb(0.68, 0.75, 0.85)),
            shortcuts,
            note_input,
        ]
        .spacing(14),
    )
    .width(Fill)
    .height(Fill)
    .padding(22)
    .style(|_| {
        container::Style::default()
            .background(Color::from_rgba(0.035, 0.055, 0.09, 0.05))
            .border(Border {
                color: Color::from_rgba(0.72, 0.82, 1.0, 0.25),
                width: 1.0,
                radius: 22.0.into(),
            })
    })
    .into()
}

fn style(_: &State, theme: &Theme) -> iced::theme::Style {
    iced::theme::Style {
        background_color: Color::TRANSPARENT,
        text_color: theme.palette().text,
    }
}

fn namespace() -> String {
    String::from("Layer Studio visibility demo")
}

fn rounded_corner_inset(y: i32, width: i32, height: i32, radius: i32) -> i32 {
    if radius == 0 || (y >= radius && y < height - radius) {
        return 0;
    }

    let row_center = if y < radius {
        y as f64 + 0.5
    } else {
        height as f64 - y as f64 - 0.5
    };
    let distance = radius as f64 - row_center;
    let remaining = (radius as f64).powi(2) - distance.powi(2);
    (radius as f64 - remaining.max(0.0).sqrt())
        .ceil()
        .clamp(0.0, (width / 2) as f64) as i32
}

fn main() -> iced_exwlshell::Result {
    application(State::default, namespace, update, view)
        .layer_settings(LayerShellSettings {
            size: LayerSize::px(WIDTH as u32, HEIGHT as u32),
            anchor: Anchor::empty(),
            blur_option: BlurOption::FullRegion,
            events_transparent: true,
            ..Default::default()
        })
        .subscription(subscription)
        .style(style)
        .run()
}
