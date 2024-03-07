mod error;
mod proxy;

use std::{borrow::Cow, sync::Arc};

use error::Error;

use iced_graphics::Compositor;

use iced_runtime::{Command, Debug, Program};

use iced_style::application::StyleSheet;

use iced_futures::{Executor, Runtime, Subscription};

use layershellev::{reexport::Anchor, LayerEvent, LayerEventError, ReturnData, WindowState};

use futures::channel::mpsc;

use std::sync::Mutex;

use crate::proxy::IcedProxy;

struct EventLoop(WindowState<()>);

impl rwh_06::HasWindowHandle for EventLoop {
    fn window_handle(&self) -> Result<rwh_06::WindowHandle<'_>, rwh_06::HandleError> {
        self.0.window_handle()
    }
}

impl rwh_06::HasDisplayHandle for EventLoop {
    fn display_handle(&self) -> Result<rwh_06::DisplayHandle<'_>, rwh_06::HandleError> {
        self.0.display_handle()
    }
}

impl EventLoop {
    fn run<F>(self, event_hander: F) -> Result<(), LayerEventError>
    where
        F: FnMut(LayerEvent<()>, &mut WindowState<()>, Option<usize>) -> ReturnData,
    {
       self.0.running(event_hander)
    }
}

/// An interactive, native cross-platform application.
///
/// This trait is the main entrypoint of Iced. Once implemented, you can run
/// your GUI application by simply calling [`run`]. It will run in
/// its own window.
///
/// An [`Application`] can execute asynchronous actions by returning a
/// [`Command`] in some of its methods.
///
/// When using an [`Application`] with the `debug` feature enabled, a debug view
/// can be toggled by pressing `F12`.
pub trait Application: Program
where
    Self::Theme: StyleSheet,
{
    /// The data needed to initialize your [`Application`].
    type Flags;

    /// Initializes the [`Application`] with the flags provided to
    /// [`run`] as part of the [`Settings`].
    ///
    /// Here is where you should return the initial state of your app.
    ///
    /// Additionally, you can return a [`Command`] if you need to perform some
    /// async action in the background on startup. This is useful if you want to
    /// load state from a file, perform an initial HTTP request, etc.
    fn new(flags: Self::Flags) -> (Self, Command<Self::Message>);

    fn namespace(&self) -> String;
    /// Returns the current title of the [`Application`].
    ///
    /// This title can be dynamic! The runtime will automatically update the
    /// title of your application when necessary.
    fn title(&self) -> String {
        self.namespace()
    }

    /// Returns the current [`Theme`] of the [`Application`].
    fn theme(&self) -> Self::Theme;

    /// Returns the [`Style`] variation of the [`Theme`].
    fn style(&self) -> <Self::Theme as StyleSheet>::Style {
        Default::default()
    }

    /// Returns the event `Subscription` for the current state of the
    /// application.
    ///
    /// The messages produced by the `Subscription` will be handled by
    /// [`update`](#tymethod.update).
    ///
    /// A `Subscription` will be kept alive as long as you keep returning it!
    ///
    /// By default, it returns an empty subscription.
    fn subscription(&self) -> Subscription<Self::Message> {
        Subscription::none()
    }

    /// Returns the scale factor of the [`Application`].
    ///
    /// It can be used to dynamically control the size of the UI at runtime
    /// (i.e. zooming).
    ///
    /// For instance, a scale factor of `2.0` will make widgets twice as big,
    /// while a scale factor of `0.5` will shrink them to half their size.
    ///
    /// By default, it returns `1.0`.
    fn scale_factor(&self) -> f64 {
        1.0
    }

    /// Defines whether or not to use natural scrolling
    fn natural_scroll(&self) -> bool {
        false
    }

    /// Returns whether the [`Application`] should be terminated.
    ///
    /// By default, it returns `false`.
    fn should_exit(&self) -> bool {
        false
    }
}

#[derive(Debug, Clone, Default)]
pub struct Settings<Flags> {
    /// The identifier of the application.
    ///
    /// If provided, this identifier may be used to identify the application or
    /// communicate with it through the windowing system.
    pub id: Option<String>,

    /// The [`window::Settings`].

    /// The data needed to initialize an [`Application`].
    ///
    /// [`Application`]: crate::Application
    pub flags: Flags,

    /// The fonts to load on boot.
    pub fonts: Vec<Cow<'static, [u8]>>,
}

// a dispatch loop, another is listen loop
pub fn run<A, E, C>(
    // TODO: settings
    settings: Settings<A::Flags>,

    compositor_settings: C::Settings,
) -> Result<(), Error>
where
    A: Application + 'static,
    E: Executor + 'static,
    C: Compositor<Renderer = A::Renderer> + 'static,
    A::Theme: StyleSheet,
{
    use futures::task;
    use futures::Future;
    let mut debug = Debug::new();
    debug.startup_started();

    let runtime: Runtime<E, IcedProxy, <A as Program>::Message> = {
        let proxy = IcedProxy;
        let executor = E::new().map_err(Error::ExecutorCreationFailed)?;

        Runtime::new(executor, proxy)
    };

    let (application, init_command) = {
        let flags = settings.flags;

        runtime.enter(|| A::new(flags))
    };

    let window: WindowState<()> = layershellev::WindowState::new(&application.namespace())
        .with_single(true)
        .with_use_display_handle(true)
        .with_layer(layershellev::reexport::Layer::Top)
        .with_anchor(Anchor::Left | Anchor::Right | Anchor::Top | Anchor::Bottom)
        .build()
        .unwrap();
    let window = Arc::new(EventLoop(window));
    let compositor = C::new(compositor_settings, window.clone())?;
    let mut renderer = compositor.create_renderer();

    for font in settings.fonts {
        use iced_core::text::Renderer;

        renderer.load_font(font);
    }

    let mut instance = Box::pin(run_instance::<A, E, C>(
        application,
        compositor,
        renderer,
        runtime,
        debug,
        init_command,
        window.clone(),
    ));

    let mut context = task::Context::from_waker(task::noop_waker_ref());

    //let _ = window.run(|event, ev, _| layershellev::ReturnData::None);
    //let (mut event_sender, event_receiver) = mpsc::unbounded();
    //let (control_sender, mut control_receiver) = mpsc::unbounded();
    todo!()
}

async fn run_instance<A, E, C>(
    mut application: A,
    mut compositor: C,
    mut renderer: A::Renderer,
    mut runtime: Runtime<E, IcedProxy, A::Message>,
    mut debug: Debug,
    init_command: Command<A::Message>,
    window: Arc<EventLoop>,
) where
    A: Application + 'static,
    E: Executor + 'static,
    C: Compositor<Renderer = A::Renderer> + 'static,
    A::Theme: StyleSheet,
{
}