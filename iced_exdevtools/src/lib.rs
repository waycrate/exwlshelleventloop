#![allow(missing_docs)]
pub use iced_debug as debug;
use iced_widget as widget;
pub use iced_widget::core;
use iced_widget::runtime;
pub use iced_widget::runtime::futures;
pub use log;
pub mod comet;
pub mod executor;
mod time_machine;

pub use crate::core::border;
pub use crate::core::keyboard;
pub use crate::core::theme::{self, Base, Theme};
pub use crate::core::time::seconds;
pub use crate::core::window;
pub use crate::core::{Alignment::Center, Color, Element, Length::Fill};
pub use crate::futures::Subscription;
pub use crate::runtime::Task;
pub use crate::widget::{
    bottom_right, button, center, column, container, horizontal_space, opaque, row, scrollable,
    stack, text, themer,
};

#[macro_export]
macro_rules! singlelayershell_dev_generate {
    (
        Type = $DevTools:ident,
        Program = $Program:ident
    ) => {
        $crate::time_machine_generate! { Program = $Program }
        struct $DevTools<P>
        where
            P: $Program,
        {
            state: P::State,
            mode: Mode,
            show_notification: bool,
            time_machine: TimeMachine<P>,
        }

        #[derive(Debug, Clone)]
        enum Message {
            HideNotification,
            ToggleComet,
            CometLaunched($crate::comet::launch::Result),
            InstallComet,
            Installing($crate::comet::install::Result),
            CancelSetup,
        }

        enum Mode {
            None,
            Setup(Setup),
        }

        enum Setup {
            Idle { goal: Goal },
            Running { logs: Vec<String> },
        }

        enum Goal {
            Installation,
            Update { revision: Option<String> },
        }

        impl<P> $DevTools<P>
        where
            P: $Program + 'static,
        {
            fn new(state: P::State) -> (Self, Task<Message>) {
                (
                    Self {
                        state,
                        mode: Mode::None,
                        show_notification: true,
                        time_machine: TimeMachine::new(),
                    },
                    $crate::executor::spawn_blocking(|mut sender| {
                        std::thread::sleep($crate::seconds(2));
                        let _ = sender.try_send(());
                    })
                    .map(|_| Message::HideNotification),
                )
            }

            fn update(&mut self, program: &P, event: Event<P>) -> Task<Event<P>> {
                match event {
                    Event::Message(message) => match message {
                        Message::HideNotification => {
                            self.show_notification = false;
                            Task::none()
                        }
                        Message::ToggleComet => {
                            if let Mode::Setup(setup) = &self.mode {
                                if matches!(setup, Setup::Idle { .. }) {
                                    self.mode = Mode::None;
                                }

                                Task::none()
                            } else if $crate::debug::quit() {
                                Task::none()
                            } else {
                                $crate::comet::launch()
                                    .map(Message::CometLaunched)
                                    .map(Event::Message)
                            }
                        }
                        Message::CometLaunched(Ok(())) => Task::none(),
                        Message::CometLaunched(Err(error)) => {
                            match error {
                                $crate::comet::launch::Error::NotFound => {
                                    self.mode = Mode::Setup(Setup::Idle {
                                        goal: Goal::Installation,
                                    });
                                }
                                $crate::comet::launch::Error::Outdated { revision } => {
                                    self.mode = Mode::Setup(Setup::Idle {
                                        goal: Goal::Update { revision },
                                    });
                                }
                                $crate::comet::launch::Error::IoFailed(error) => {
                                    $crate::log::error!("comet failed to run: {error}");
                                }
                            }

                            Task::none()
                        }
                        Message::InstallComet => {
                            self.mode = Mode::Setup(Setup::Running { logs: Vec::new() });

                            $crate::comet::install()
                                .map(Message::Installing)
                                .map(Event::Message)
                        }

                        Message::Installing(Ok(installation)) => {
                            let Mode::Setup(Setup::Running { logs }) = &mut self.mode else {
                                return Task::none();
                            };

                            match installation {
                                $crate::comet::install::Event::Logged(log) => {
                                    logs.push(log);
                                    Task::none()
                                }
                                $crate::comet::install::Event::Finished => {
                                    self.mode = Mode::None;
                                    $crate::comet::launch().discard()
                                }
                            }
                        }
                        Message::Installing(Err(error)) => {
                            let Mode::Setup(Setup::Running { logs }) = &mut self.mode else {
                                return Task::none();
                            };

                            match error {
                                $crate::comet::install::Error::ProcessFailed(status) => {
                                    logs.push(format!("process failed with {status}"));
                                }
                                $crate::comet::install::Error::IoFailed(error) => {
                                    logs.push(error.to_string());
                                }
                            }

                            Task::none()
                        }
                        Message::CancelSetup => {
                            self.mode = Mode::None;

                            Task::none()
                        }
                    },
                    Event::Program(message) => {
                        self.time_machine.push(&message);

                        if self.time_machine.is_rewinding() {
                            $crate::debug::enable();
                        }

                        let span = $crate::debug::update(&message);
                        let task = program.update(&mut self.state, message);
                        $crate::debug::tasks_spawned(task.units());
                        span.finish();

                        if self.time_machine.is_rewinding() {
                            $crate::debug::disable();
                        }

                        task.map(Event::Program)
                    }
                    Event::Command(command) => {
                        match command {
                            $crate::debug::Command::RewindTo { message } => {
                                self.time_machine.rewind(program, message);
                            }
                            $crate::debug::Command::GoLive => {
                                self.time_machine.go_to_present();
                            }
                        }

                        Task::none()
                    }
                    Event::Discard => Task::none(),
                }
            }

            fn view(&self, program: &P) -> $crate::Element<'_, Event<P>, P::Theme, P::Renderer> {
                let state = self.state();

                let view = {
                    let view = program.view(state);

                    if self.time_machine.is_rewinding() {
                        view.map(|_| Event::Discard)
                    } else {
                        view.map(Event::Program)
                    }
                };

                let theme = program.theme(state);

                let derive_theme = move || {
                    theme
                        .palette()
                        .map(|palette| $crate::Theme::custom("iced devtools", palette))
                        .unwrap_or_default()
                };

                let mode = match &self.mode {
                    Mode::None => None,
                    Mode::Setup(setup) => {
                        let stage: Element<'_, _, $crate::Theme, P::Renderer> = match setup {
                            Setup::Idle { goal } => self::setup(goal),
                            Setup::Running { logs } => installation(logs),
                        };

                        let setup = $crate::center(
                            $crate::container(stage)
                                .padding(20)
                                .max_width(500)
                                .style($crate::container::bordered_box),
                        )
                        .padding(10)
                        .style(|_theme| {
                            $crate::container::Style::default()
                                .background($crate::Color::BLACK.scale_alpha(0.8))
                        });

                        Some(setup)
                    }
                }
                .map(|mode| {
                    $crate::themer(derive_theme(), Element::from(mode).map(Event::Message))
                });

                let notification = self.show_notification.then(|| {
                    $crate::themer(
                        derive_theme(),
                        $crate::bottom_right($crate::opaque(
                            $crate::container($crate::text("Press F12 to open debug metrics"))
                                .padding(10)
                                .style($crate::container::dark),
                        )),
                    )
                });

                $crate::stack![view]
                    .height($crate::Fill)
                    .push_maybe(mode.map($crate::opaque))
                    .push_maybe(notification)
                    .into()
            }

            fn subscription(&self, program: &P) -> $crate::Subscription<Event<P>> {
                let subscription = program.subscription(&self.state).map(Event::Program);
                $crate::debug::subscriptions_tracked(subscription.units());

                let hotkeys =
                    $crate::futures::keyboard::on_key_press(|key, _modifiers| match key {
                        $crate::keyboard::Key::Named($crate::keyboard::key::Named::F12) => {
                            Some(Message::ToggleComet)
                        }
                        _ => None,
                    })
                    .map(Event::Message);

                let commands = $crate::debug::commands().map(Event::Command);

                $crate::Subscription::batch([subscription, hotkeys, commands])
            }

            fn state(&self) -> &P::State {
                self.time_machine.state().unwrap_or(&self.state)
            }
        }

        enum Event<P>
        where
            P: $Program,
        {
            Message(Message),
            Program(P::Message),
            Command($crate::debug::Command),
            Discard,
        }

        impl<P> std::fmt::Debug for Event<P>
        where
            P: $Program,
        {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                match self {
                    Self::Message(message) => message.fmt(f),
                    Self::Program(message) => message.fmt(f),
                    Self::Command(command) => command.fmt(f),
                    Self::Discard => f.write_str("Discard"),
                }
            }
        }
        #[cfg(feature = "time-travel")]
        impl<P> Clone for Event<P>
        where
            P: $Program,
        {
            fn clone(&self) -> Self {
                match self {
                    Self::Message(message) => Self::Message(message.clone()),
                    Self::Program(message) => Self::Program(message.clone()),
                    Self::Command(command) => Self::Command(*command),
                    Self::Discard => Self::Discard,
                }
            }
        }

        fn setup<Renderer>(goal: &Goal) -> $crate::Element<'_, Message, $crate::Theme, Renderer>
        where
            Renderer: $crate::core::text::Renderer + 'static,
        {
            let controls = $crate::row![
                $crate::button($crate::text("Cancel").center().width($crate::Fill))
                    .width(100)
                    .on_press(Message::CancelSetup)
                    .style($crate::button::danger),
                $crate::horizontal_space(),
                $crate::button(
                    $crate::text(match goal {
                        Goal::Installation => "Install",
                        Goal::Update { .. } => "Update",
                    })
                    .center()
                    .width($crate::Fill)
                )
                .width(100)
                .on_press(Message::InstallComet)
                .style($crate::button::success),
            ];

            let command = $crate::container(
                $crate::text!(
                    "cargo install --locked \\
            --git https://github.com/iced-rs/comet.git \\
            --rev {}",
                    $crate::comet::COMPATIBLE_REVISION
                )
                .size(14)
                .font(Renderer::MONOSPACE_FONT),
            )
            .width($crate::Fill)
            .padding(5)
            .style($crate::container::dark);

            Element::from(match goal {
                Goal::Installation => $crate::column![
                    $crate::text("comet is not installed!").size(20),
                    "In order to display performance \
                        metrics, the  comet debugger must \
                        be installed in your system.",
                    "The comet debugger is an official \
                        companion tool that helps you debug \
                        your iced applications.",
                    $crate::column![
                        "Do you wish to install it with the \
                            following  command?",
                        command
                    ]
                    .spacing(10),
                    controls,
                ]
                .spacing(20),
                Goal::Update { revision } => {
                    let comparison = $crate::column![
                        $crate::row![
                            "Installed revision:",
                            $crate::horizontal_space(),
                            inline_code(revision.as_deref().unwrap_or("Unknown"))
                        ]
                        .align_y($crate::Center),
                        $crate::row![
                            "Compatible revision:",
                            $crate::horizontal_space(),
                            inline_code($crate::comet::COMPATIBLE_REVISION),
                        ]
                        .align_y($crate::Center)
                    ]
                    .spacing(5);

                    $crate::column![
                        $crate::text("comet is out of date!").size(20),
                        comparison,
                        $crate::column![
                            "Do you wish to update it with the following \
                                command?",
                            command
                        ]
                        .spacing(10),
                        controls,
                    ]
                    .spacing(20)
                }
            })
        }

        fn installation<'a, Renderer>(
            logs: &'a [String],
        ) -> $crate::Element<'a, Message, $crate::Theme, Renderer>
        where
            Renderer: $crate::core::text::Renderer + 'a,
        {
            $crate::column![
                $crate::text("Installing comet...").size(20),
                $crate::container(
                    $crate::scrollable(
                        $crate::column(logs.iter().map(|log| {
                            $crate::text(log)
                                .size(12)
                                .font(Renderer::MONOSPACE_FONT)
                                .into()
                        }),)
                        .spacing(3),
                    )
                    .spacing(10)
                    .width($crate::Fill)
                    .height(300)
                    .anchor_bottom(),
                )
                .padding(10)
                .style($crate::container::dark)
            ]
            .spacing(20)
            .into()
        }

        fn inline_code<'a, Renderer>(
            code: impl $crate::text::IntoFragment<'a>,
        ) -> $crate::Element<'a, Message, $crate::Theme, Renderer>
        where
            Renderer: $crate::core::text::Renderer + 'a,
        {
            $crate::container($crate::text(code).font(Renderer::MONOSPACE_FONT).size(12))
                .style(|_theme| {
                    $crate::container::Style::default()
                        .background($crate::Color::BLACK)
                        .border($crate::border::rounded(2))
                })
                .padding([2, 4])
                .into()
        }
    };
}

#[macro_export]
macro_rules! multilayershell_dev_generate {
    (
        Type = $DevTools:ident,
        Program = $Program:ident
    ) => {
        $crate::time_machine_generate! { Program = $Program }
        struct $DevTools<P>
        where
            P: $Program,
        {
            state: P::State,
            mode: Mode,
            show_notification: bool,
            time_machine: TimeMachine<P>,
        }

        #[derive(Debug, Clone)]
        enum Message {
            HideNotification,
            ToggleComet,
            CometLaunched($crate::comet::launch::Result),
            InstallComet,
            Installing($crate::comet::install::Result),
            CancelSetup,
        }

        enum Mode {
            None,
            Setup(Setup),
        }

        enum Setup {
            Idle { goal: Goal },
            Running { logs: Vec<String> },
        }

        enum Goal {
            Installation,
            Update { revision: Option<String> },
        }

        impl<P> $DevTools<P>
        where
            P: $Program + 'static,
        {
            fn new(state: P::State) -> (Self, Task<Message>) {
                (
                    Self {
                        state,
                        mode: Mode::None,
                        show_notification: true,
                        time_machine: TimeMachine::new(),
                    },
                    $crate::executor::spawn_blocking(|mut sender| {
                        std::thread::sleep($crate::seconds(2));
                        let _ = sender.try_send(());
                    })
                    .map(|_| Message::HideNotification),
                )
            }
            //fn title(&self, program: &P, window: $crate::window::Id) -> String {
            //    program.title(&self.state, window)
            //}
            fn update(&mut self, program: &P, event: Event<P>) -> Task<Event<P>> {
                match event {
                    Event::Message(message) => match message {
                        Message::HideNotification => {
                            self.show_notification = false;
                            Task::none()
                        }
                        Message::ToggleComet => {
                            if let Mode::Setup(setup) = &self.mode {
                                if matches!(setup, Setup::Idle { .. }) {
                                    self.mode = Mode::None;
                                }

                                Task::none()
                            } else if $crate::debug::quit() {
                                Task::none()
                            } else {
                                $crate::comet::launch()
                                    .map(Message::CometLaunched)
                                    .map(Event::Message)
                            }
                        }
                        Message::CometLaunched(Ok(())) => Task::none(),
                        Message::CometLaunched(Err(error)) => {
                            match error {
                                $crate::comet::launch::Error::NotFound => {
                                    self.mode = Mode::Setup(Setup::Idle {
                                        goal: Goal::Installation,
                                    });
                                }
                                $crate::comet::launch::Error::Outdated { revision } => {
                                    self.mode = Mode::Setup(Setup::Idle {
                                        goal: Goal::Update { revision },
                                    });
                                }
                                $crate::comet::launch::Error::IoFailed(error) => {
                                    $crate::log::error!("comet failed to run: {error}");
                                }
                            }

                            Task::none()
                        }
                        Message::InstallComet => {
                            self.mode = Mode::Setup(Setup::Running { logs: Vec::new() });

                            $crate::comet::install()
                                .map(Message::Installing)
                                .map(Event::Message)
                        }

                        Message::Installing(Ok(installation)) => {
                            let Mode::Setup(Setup::Running { logs }) = &mut self.mode else {
                                return Task::none();
                            };

                            match installation {
                                $crate::comet::install::Event::Logged(log) => {
                                    logs.push(log);
                                    Task::none()
                                }
                                $crate::comet::install::Event::Finished => {
                                    self.mode = Mode::None;
                                    $crate::comet::launch().discard()
                                }
                            }
                        }
                        Message::Installing(Err(error)) => {
                            let Mode::Setup(Setup::Running { logs }) = &mut self.mode else {
                                return Task::none();
                            };

                            match error {
                                $crate::comet::install::Error::ProcessFailed(status) => {
                                    logs.push(format!("process failed with {status}"));
                                }
                                $crate::comet::install::Error::IoFailed(error) => {
                                    logs.push(error.to_string());
                                }
                            }

                            Task::none()
                        }
                        Message::CancelSetup => {
                            self.mode = Mode::None;

                            Task::none()
                        }
                    },
                    Event::Program(message) => {
                        self.time_machine.push(&message);

                        if self.time_machine.is_rewinding() {
                            $crate::debug::enable();
                        }

                        let span = $crate::debug::update(&message);
                        let task = program.update(&mut self.state, message);
                        $crate::debug::tasks_spawned(task.units());
                        span.finish();

                        if self.time_machine.is_rewinding() {
                            $crate::debug::disable();
                        }

                        task.map(Event::Program)
                    }
                    Event::Command(command) => {
                        match command {
                            $crate::debug::Command::RewindTo { message } => {
                                self.time_machine.rewind(program, message);
                            }
                            $crate::debug::Command::GoLive => {
                                self.time_machine.go_to_present();
                            }
                        }

                        Task::none()
                    }
                    Event::Discard => Task::none(),
                }
            }

            fn view(
                &self,
                program: &P,
                window: $crate::core::window::Id,
            ) -> $crate::Element<'_, Event<P>, P::Theme, P::Renderer> {
                let state = self.state();

                let view = {
                    let view = program.view(state, window);

                    if self.time_machine.is_rewinding() {
                        view.map(|_| Event::Discard)
                    } else {
                        view.map(Event::Program)
                    }
                };

                let theme = program.theme(state, window);

                let derive_theme = move || {
                    theme
                        .palette()
                        .map(|palette| $crate::Theme::custom("iced devtools", palette))
                        .unwrap_or_default()
                };

                let mode = match &self.mode {
                    Mode::None => None,
                    Mode::Setup(setup) => {
                        let stage: Element<'_, _, $crate::Theme, P::Renderer> = match setup {
                            Setup::Idle { goal } => self::setup(goal),
                            Setup::Running { logs } => installation(logs),
                        };

                        let setup = $crate::center(
                            $crate::container(stage)
                                .padding(20)
                                .max_width(500)
                                .style($crate::container::bordered_box),
                        )
                        .padding(10)
                        .style(|_theme| {
                            $crate::container::Style::default()
                                .background($crate::Color::BLACK.scale_alpha(0.8))
                        });

                        Some(setup)
                    }
                }
                .map(|mode| {
                    $crate::themer(derive_theme(), Element::from(mode).map(Event::Message))
                });

                let notification = self.show_notification.then(|| {
                    $crate::themer(
                        derive_theme(),
                        $crate::bottom_right($crate::opaque(
                            $crate::container($crate::text("Press F12 to open debug metrics"))
                                .padding(10)
                                .style($crate::container::dark),
                        )),
                    )
                });

                $crate::stack![view]
                    .height($crate::Fill)
                    .push_maybe(mode.map($crate::opaque))
                    .push_maybe(notification)
                    .into()
            }

            fn subscription(&self, program: &P) -> $crate::Subscription<Event<P>> {
                let subscription = program.subscription(&self.state).map(Event::Program);
                $crate::debug::subscriptions_tracked(subscription.units());

                let hotkeys =
                    $crate::futures::keyboard::on_key_press(|key, _modifiers| match key {
                        $crate::keyboard::Key::Named($crate::keyboard::key::Named::F12) => {
                            Some(Message::ToggleComet)
                        }
                        _ => None,
                    })
                    .map(Event::Message);

                let commands = $crate::debug::commands().map(Event::Command);

                $crate::Subscription::batch([subscription, hotkeys, commands])
            }

            fn state(&self) -> &P::State {
                self.time_machine.state().unwrap_or(&self.state)
            }

            fn state_mut(&mut self) -> &mut P::State {
                self.time_machine.state_mut().unwrap_or(&mut self.state)
            }
        }

        enum Event<P>
        where
            P: $Program,
        {
            Message(Message),
            Program(P::Message),
            Command($crate::debug::Command),
            Discard,
        }

        impl<P> std::fmt::Debug for Event<P>
        where
            P: $Program,
        {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                match self {
                    Self::Message(message) => message.fmt(f),
                    Self::Program(message) => message.fmt(f),
                    Self::Command(command) => command.fmt(f),
                    Self::Discard => f.write_str("Discard"),
                }
            }
        }
        #[cfg(feature = "time-travel")]
        impl<P> Clone for Event<P>
        where
            P: $Program,
        {
            fn clone(&self) -> Self {
                match self {
                    Self::Message(message) => Self::Message(message.clone()),
                    Self::Program(message) => Self::Program(message.clone()),
                    Self::Command(command) => Self::Command(*command),
                    Self::Discard => Self::Discard,
                }
            }
        }

        fn setup<Renderer>(goal: &Goal) -> $crate::Element<'_, Message, $crate::Theme, Renderer>
        where
            Renderer: $crate::core::text::Renderer + 'static,
        {
            let controls = $crate::row![
                $crate::button($crate::text("Cancel").center().width($crate::Fill))
                    .width(100)
                    .on_press(Message::CancelSetup)
                    .style($crate::button::danger),
                $crate::horizontal_space(),
                $crate::button(
                    $crate::text(match goal {
                        Goal::Installation => "Install",
                        Goal::Update { .. } => "Update",
                    })
                    .center()
                    .width($crate::Fill)
                )
                .width(100)
                .on_press(Message::InstallComet)
                .style($crate::button::success),
            ];

            let command = $crate::container(
                $crate::text!(
                    "cargo install --locked \\
            --git https://github.com/iced-rs/comet.git \\
            --rev {}",
                    $crate::comet::COMPATIBLE_REVISION
                )
                .size(14)
                .font(Renderer::MONOSPACE_FONT),
            )
            .width($crate::Fill)
            .padding(5)
            .style($crate::container::dark);

            Element::from(match goal {
                Goal::Installation => $crate::column![
                    $crate::text("comet is not installed!").size(20),
                    "In order to display performance \
                        metrics, the  comet debugger must \
                        be installed in your system.",
                    "The comet debugger is an official \
                        companion tool that helps you debug \
                        your iced applications.",
                    $crate::column![
                        "Do you wish to install it with the \
                            following  command?",
                        command
                    ]
                    .spacing(10),
                    controls,
                ]
                .spacing(20),
                Goal::Update { revision } => {
                    let comparison = $crate::column![
                        $crate::row![
                            "Installed revision:",
                            $crate::horizontal_space(),
                            inline_code(revision.as_deref().unwrap_or("Unknown"))
                        ]
                        .align_y($crate::Center),
                        $crate::row![
                            "Compatible revision:",
                            $crate::horizontal_space(),
                            inline_code($crate::comet::COMPATIBLE_REVISION),
                        ]
                        .align_y($crate::Center)
                    ]
                    .spacing(5);

                    $crate::column![
                        $crate::text("comet is out of date!").size(20),
                        comparison,
                        $crate::column![
                            "Do you wish to update it with the following \
                                command?",
                            command
                        ]
                        .spacing(10),
                        controls,
                    ]
                    .spacing(20)
                }
            })
        }

        fn installation<'a, Renderer>(
            logs: &'a [String],
        ) -> $crate::Element<'a, Message, $crate::Theme, Renderer>
        where
            Renderer: $crate::core::text::Renderer + 'a,
        {
            $crate::column![
                $crate::text("Installing comet...").size(20),
                $crate::container(
                    $crate::scrollable(
                        $crate::column(logs.iter().map(|log| {
                            $crate::text(log)
                                .size(12)
                                .font(Renderer::MONOSPACE_FONT)
                                .into()
                        }),)
                        .spacing(3),
                    )
                    .spacing(10)
                    .width($crate::Fill)
                    .height(300)
                    .anchor_bottom(),
                )
                .padding(10)
                .style($crate::container::dark)
            ]
            .spacing(20)
            .into()
        }

        fn inline_code<'a, Renderer>(
            code: impl $crate::text::IntoFragment<'a>,
        ) -> $crate::Element<'a, Message, $crate::Theme, Renderer>
        where
            Renderer: $crate::core::text::Renderer + 'a,
        {
            $crate::container($crate::text(code).font(Renderer::MONOSPACE_FONT).size(12))
                .style(|_theme| {
                    $crate::container::Style::default()
                        .background($crate::Color::BLACK)
                        .border($crate::border::rounded(2))
                })
                .padding([2, 4])
                .into()
        }
    };
}

#[macro_export]
macro_rules! sessionlock_dev_generate {
    (
        Type = $DevTools:ident,
        Program = $Program:ident
    ) => {
        $crate::time_machine_generate! { Program = $Program }
        struct $DevTools<P>
        where
            P: $Program,
        {
            state: P::State,
            mode: Mode,
            show_notification: bool,
            time_machine: TimeMachine<P>,
        }

        #[derive(Debug, Clone)]
        enum Message {
            HideNotification,
            ToggleComet,
            CometLaunched($crate::comet::launch::Result),
            InstallComet,
            Installing($crate::comet::install::Result),
            CancelSetup,
        }

        enum Mode {
            None,
            Setup(Setup),
        }

        enum Setup {
            Idle { goal: Goal },
            Running { logs: Vec<String> },
        }

        enum Goal {
            Installation,
            Update { revision: Option<String> },
        }

        impl<P> $DevTools<P>
        where
            P: $Program + 'static,
        {
            fn new(state: P::State) -> (Self, Task<Message>) {
                (
                    Self {
                        state,
                        mode: Mode::None,
                        show_notification: true,
                        time_machine: TimeMachine::new(),
                    },
                    $crate::executor::spawn_blocking(|mut sender| {
                        std::thread::sleep($crate::seconds(2));
                        let _ = sender.try_send(());
                    })
                    .map(|_| Message::HideNotification),
                )
            }
            //fn title(&self, program: &P, window: $crate::window::Id) -> String {
            //    program.title(&self.state, window)
            //}
            fn update(&mut self, program: &P, event: Event<P>) -> Task<Event<P>> {
                match event {
                    Event::Message(message) => match message {
                        Message::HideNotification => {
                            self.show_notification = false;
                            Task::none()
                        }
                        Message::ToggleComet => {
                            if let Mode::Setup(setup) = &self.mode {
                                if matches!(setup, Setup::Idle { .. }) {
                                    self.mode = Mode::None;
                                }

                                Task::none()
                            } else if $crate::debug::quit() {
                                Task::none()
                            } else {
                                $crate::comet::launch()
                                    .map(Message::CometLaunched)
                                    .map(Event::Message)
                            }
                        }
                        Message::CometLaunched(Ok(())) => Task::none(),
                        Message::CometLaunched(Err(error)) => {
                            match error {
                                $crate::comet::launch::Error::NotFound => {
                                    self.mode = Mode::Setup(Setup::Idle {
                                        goal: Goal::Installation,
                                    });
                                }
                                $crate::comet::launch::Error::Outdated { revision } => {
                                    self.mode = Mode::Setup(Setup::Idle {
                                        goal: Goal::Update { revision },
                                    });
                                }
                                $crate::comet::launch::Error::IoFailed(error) => {
                                    $crate::log::error!("comet failed to run: {error}");
                                }
                            }

                            Task::none()
                        }
                        Message::InstallComet => {
                            self.mode = Mode::Setup(Setup::Running { logs: Vec::new() });

                            $crate::comet::install()
                                .map(Message::Installing)
                                .map(Event::Message)
                        }

                        Message::Installing(Ok(installation)) => {
                            let Mode::Setup(Setup::Running { logs }) = &mut self.mode else {
                                return Task::none();
                            };

                            match installation {
                                $crate::comet::install::Event::Logged(log) => {
                                    logs.push(log);
                                    Task::none()
                                }
                                $crate::comet::install::Event::Finished => {
                                    self.mode = Mode::None;
                                    $crate::comet::launch().discard()
                                }
                            }
                        }
                        Message::Installing(Err(error)) => {
                            let Mode::Setup(Setup::Running { logs }) = &mut self.mode else {
                                return Task::none();
                            };

                            match error {
                                $crate::comet::install::Error::ProcessFailed(status) => {
                                    logs.push(format!("process failed with {status}"));
                                }
                                $crate::comet::install::Error::IoFailed(error) => {
                                    logs.push(error.to_string());
                                }
                            }

                            Task::none()
                        }
                        Message::CancelSetup => {
                            self.mode = Mode::None;

                            Task::none()
                        }
                    },
                    Event::Program(message) => {
                        self.time_machine.push(&message);

                        if self.time_machine.is_rewinding() {
                            $crate::debug::enable();
                        }

                        let span = $crate::debug::update(&message);
                        let task = program.update(&mut self.state, message);
                        $crate::debug::tasks_spawned(task.units());
                        span.finish();

                        if self.time_machine.is_rewinding() {
                            $crate::debug::disable();
                        }

                        task.map(Event::Program)
                    }
                    Event::Command(command) => {
                        match command {
                            $crate::debug::Command::RewindTo { message } => {
                                self.time_machine.rewind(program, message);
                            }
                            $crate::debug::Command::GoLive => {
                                self.time_machine.go_to_present();
                            }
                        }

                        Task::none()
                    }
                    Event::Discard => Task::none(),
                }
            }

            fn view(
                &self,
                program: &P,
                window: $crate::core::window::Id,
            ) -> $crate::Element<'_, Event<P>, P::Theme, P::Renderer> {
                let state = self.state();

                let view = {
                    let view = program.view(state, window);

                    if self.time_machine.is_rewinding() {
                        view.map(|_| Event::Discard)
                    } else {
                        view.map(Event::Program)
                    }
                };

                let theme = program.theme(state);

                let derive_theme = move || {
                    theme
                        .palette()
                        .map(|palette| $crate::Theme::custom("iced devtools", palette))
                        .unwrap_or_default()
                };

                let mode = match &self.mode {
                    Mode::None => None,
                    Mode::Setup(setup) => {
                        let stage: Element<'_, _, $crate::Theme, P::Renderer> = match setup {
                            Setup::Idle { goal } => self::setup(goal),
                            Setup::Running { logs } => installation(logs),
                        };

                        let setup = $crate::center(
                            $crate::container(stage)
                                .padding(20)
                                .max_width(500)
                                .style($crate::container::bordered_box),
                        )
                        .padding(10)
                        .style(|_theme| {
                            $crate::container::Style::default()
                                .background($crate::Color::BLACK.scale_alpha(0.8))
                        });

                        Some(setup)
                    }
                }
                .map(|mode| {
                    $crate::themer(derive_theme(), Element::from(mode).map(Event::Message))
                });

                let notification = self.show_notification.then(|| {
                    $crate::themer(
                        derive_theme(),
                        $crate::bottom_right($crate::opaque(
                            $crate::container($crate::text("Press F12 to open debug metrics"))
                                .padding(10)
                                .style($crate::container::dark),
                        )),
                    )
                });

                $crate::stack![view]
                    .height($crate::Fill)
                    .push_maybe(mode.map($crate::opaque))
                    .push_maybe(notification)
                    .into()
            }

            fn subscription(&self, program: &P) -> $crate::Subscription<Event<P>> {
                let subscription = program.subscription(&self.state).map(Event::Program);
                $crate::debug::subscriptions_tracked(subscription.units());

                let hotkeys =
                    $crate::futures::keyboard::on_key_press(|key, _modifiers| match key {
                        $crate::keyboard::Key::Named($crate::keyboard::key::Named::F12) => {
                            Some(Message::ToggleComet)
                        }
                        _ => None,
                    })
                    .map(Event::Message);

                let commands = $crate::debug::commands().map(Event::Command);

                $crate::Subscription::batch([subscription, hotkeys, commands])
            }

            fn state(&self) -> &P::State {
                self.time_machine.state().unwrap_or(&self.state)
            }

            fn state_mut(&mut self) -> &mut P::State {
                self.time_machine.state_mut().unwrap_or(&mut self.state)
            }
        }

        enum Event<P>
        where
            P: $Program,
        {
            Message(Message),
            Program(P::Message),
            Command($crate::debug::Command),
            Discard,
        }

        impl<P> std::fmt::Debug for Event<P>
        where
            P: $Program,
        {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                match self {
                    Self::Message(message) => message.fmt(f),
                    Self::Program(message) => message.fmt(f),
                    Self::Command(command) => command.fmt(f),
                    Self::Discard => f.write_str("Discard"),
                }
            }
        }
        #[cfg(feature = "time-travel")]
        impl<P> Clone for Event<P>
        where
            P: $Program,
        {
            fn clone(&self) -> Self {
                match self {
                    Self::Message(message) => Self::Message(message.clone()),
                    Self::Program(message) => Self::Program(message.clone()),
                    Self::Command(command) => Self::Command(*command),
                    Self::Discard => Self::Discard,
                }
            }
        }

        fn setup<Renderer>(goal: &Goal) -> $crate::Element<'_, Message, $crate::Theme, Renderer>
        where
            Renderer: $crate::core::text::Renderer + 'static,
        {
            let controls = $crate::row![
                $crate::button($crate::text("Cancel").center().width($crate::Fill))
                    .width(100)
                    .on_press(Message::CancelSetup)
                    .style($crate::button::danger),
                $crate::horizontal_space(),
                $crate::button(
                    $crate::text(match goal {
                        Goal::Installation => "Install",
                        Goal::Update { .. } => "Update",
                    })
                    .center()
                    .width($crate::Fill)
                )
                .width(100)
                .on_press(Message::InstallComet)
                .style($crate::button::success),
            ];

            let command = $crate::container(
                $crate::text!(
                    "cargo install --locked \\
            --git https://github.com/iced-rs/comet.git \\
            --rev {}",
                    $crate::comet::COMPATIBLE_REVISION
                )
                .size(14)
                .font(Renderer::MONOSPACE_FONT),
            )
            .width($crate::Fill)
            .padding(5)
            .style($crate::container::dark);

            Element::from(match goal {
                Goal::Installation => $crate::column![
                    $crate::text("comet is not installed!").size(20),
                    "In order to display performance \
                        metrics, the  comet debugger must \
                        be installed in your system.",
                    "The comet debugger is an official \
                        companion tool that helps you debug \
                        your iced applications.",
                    $crate::column![
                        "Do you wish to install it with the \
                            following  command?",
                        command
                    ]
                    .spacing(10),
                    controls,
                ]
                .spacing(20),
                Goal::Update { revision } => {
                    let comparison = $crate::column![
                        $crate::row![
                            "Installed revision:",
                            $crate::horizontal_space(),
                            inline_code(revision.as_deref().unwrap_or("Unknown"))
                        ]
                        .align_y($crate::Center),
                        $crate::row![
                            "Compatible revision:",
                            $crate::horizontal_space(),
                            inline_code($crate::comet::COMPATIBLE_REVISION),
                        ]
                        .align_y($crate::Center)
                    ]
                    .spacing(5);

                    $crate::column![
                        $crate::text("comet is out of date!").size(20),
                        comparison,
                        $crate::column![
                            "Do you wish to update it with the following \
                                command?",
                            command
                        ]
                        .spacing(10),
                        controls,
                    ]
                    .spacing(20)
                }
            })
        }

        fn installation<'a, Renderer>(
            logs: &'a [String],
        ) -> $crate::Element<'a, Message, $crate::Theme, Renderer>
        where
            Renderer: $crate::core::text::Renderer + 'a,
        {
            $crate::column![
                $crate::text("Installing comet...").size(20),
                $crate::container(
                    $crate::scrollable(
                        $crate::column(logs.iter().map(|log| {
                            $crate::text(log)
                                .size(12)
                                .font(Renderer::MONOSPACE_FONT)
                                .into()
                        }),)
                        .spacing(3),
                    )
                    .spacing(10)
                    .width($crate::Fill)
                    .height(300)
                    .anchor_bottom(),
                )
                .padding(10)
                .style($crate::container::dark)
            ]
            .spacing(20)
            .into()
        }

        fn inline_code<'a, Renderer>(
            code: impl $crate::text::IntoFragment<'a>,
        ) -> $crate::Element<'a, Message, $crate::Theme, Renderer>
        where
            Renderer: $crate::core::text::Renderer + 'a,
        {
            $crate::container($crate::text(code).font(Renderer::MONOSPACE_FONT).size(12))
                .style(|_theme| {
                    $crate::container::Style::default()
                        .background($crate::Color::BLACK)
                        .border($crate::border::rounded(2))
                })
                .padding([2, 4])
                .into()
        }
    };
}
