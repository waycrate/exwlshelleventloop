use super::{BootFn, NameSpace, SingleApplication, ViewFn};
use crate::DefaultStyle;
use iced_core::Element;
use iced_debug as debug;
use iced_futures::Subscription;
use iced_program::Program;
use iced_runtime::Task;
use std::time::Instant;

/// Creates an [`SingleApplication`] with an `update` function that also
/// takes the [`Instant`] of each `Message`.
///
/// This constructor is useful to create animated applications that
/// are _pure_ (e.g. without relying on side-effect calls like [`Instant::now`]).
///
/// Purity is needed when you want your application to end up in the
/// same exact state given the same history of messages. This property
/// enables proper time traveling debugging with [`comet`].
///
/// [`comet`]: https://github.com/iced-rs/comet
pub fn timed<State, Message, Theme, Renderer>(
    boot: impl BootFn<State, Message>,
    namespace: impl NameSpace,
    update: impl UpdateFn<State, Message>,
    subscription: impl Fn(&State) -> Subscription<Message>,
    view: impl for<'a> ViewFn<'a, State, Message, Theme, Renderer>,
) -> SingleApplication<impl Program<State = State, Message = (Message, Instant), Theme = Theme>>
where
    State: 'static,
    Message: Send + 'static,
    Theme: DefaultStyle + 'static,
    Renderer: iced_program::Renderer + 'static,
{
    use std::marker::PhantomData;

    struct Instance<State, Message, Theme, Renderer, BootFn, Update, Subscription, ViewFn> {
        boot: BootFn,
        update: Update,
        subscription: Subscription,
        view: ViewFn,
        _state: PhantomData<State>,
        _message: PhantomData<Message>,
        _theme: PhantomData<Theme>,
        _renderer: PhantomData<Renderer>,
    }

    impl<State, Message, Theme, Renderer, BootFn, Update, Subscription, ViewFn> Program
        for Instance<State, Message, Theme, Renderer, BootFn, Update, Subscription, ViewFn>
    where
        Message: Send + 'static,
        Theme: DefaultStyle + 'static,
        Renderer: iced_program::Renderer + 'static,
        BootFn: self::BootFn<State, Message>,
        Update: self::UpdateFn<State, Message>,
        Subscription: Fn(&State) -> self::Subscription<Message>,
        ViewFn: for<'a> self::ViewFn<'a, State, Message, Theme, Renderer>,
    {
        type State = State;
        type Message = (Message, Instant);
        type Theme = Theme;
        type Renderer = Renderer;
        type Executor = iced_futures::backend::default::Executor;

        fn name() -> &'static str {
            let name = std::any::type_name::<State>();

            name.split("::").next().unwrap_or("a_cool_application")
        }

        fn settings(&self) -> iced_core::Settings {
            Default::default()
        }

        fn window(&self) -> Option<iced_core::window::Settings> {
            None
        }

        fn boot(&self) -> (State, Task<Self::Message>) {
            let (state, task) = self.boot.boot();

            (state, task.map(|message| (message, Instant::now())))
        }

        fn update(
            &self,
            state: &mut Self::State,
            (message, now): Self::Message,
        ) -> Task<Self::Message> {
            debug::hot(move || {
                self.update
                    .update(state, message, now)
                    .into()
                    .map(|message| (message, Instant::now()))
            })
        }

        fn view<'a>(
            &self,
            state: &'a Self::State,
            _window: iced_core::window::Id,
        ) -> Element<'a, Self::Message, Self::Theme, Self::Renderer> {
            debug::hot(|| {
                self.view
                    .view(state)
                    .map(|message| (message, Instant::now()))
            })
        }

        fn subscription(&self, state: &Self::State) -> self::Subscription<Self::Message> {
            debug::hot(|| (self.subscription)(state).map(|message| (message, Instant::now())))
        }
    }

    SingleApplication {
        raw: Instance {
            boot,
            update,
            subscription,
            view,
            _state: PhantomData,
            _message: PhantomData,
            _theme: PhantomData,
            _renderer: PhantomData,
        },
        settings: crate::Settings::default(),
        namespace: namespace.namespace(),
    }
}

/// The update logic of some timed [`Application`].
///
/// This is like [`application::UpdateFn`](super::UpdateFn),
/// but it also takes an [`Instant`].
pub trait UpdateFn<State, Message> {
    /// Processes the message and updates the state of the [`Application`].
    fn update(&self, state: &mut State, message: Message, now: Instant)
    -> impl Into<Task<Message>>;
}

impl<State, Message> UpdateFn<State, Message> for () {
    fn update(
        &self,
        _state: &mut State,
        _message: Message,
        _now: Instant,
    ) -> impl Into<Task<Message>> {
    }
}

impl<T, State, Message, C> UpdateFn<State, Message> for T
where
    T: Fn(&mut State, Message, Instant) -> C,
    C: Into<Task<Message>>,
{
    fn update(
        &self,
        state: &mut State,
        message: Message,
        now: Instant,
    ) -> impl Into<Task<Message>> {
        self(state, message, now)
    }
}
