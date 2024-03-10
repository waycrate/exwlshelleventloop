mod state;

use std::{mem::ManuallyDrop, sync::Arc};

use crate::{clipboard::LayerShellClipboard, error::Error};

use iced_graphics::Compositor;
use state::State;

use iced_core::{
    mouse as IcedCoreMouse, time::Instant, window as IcedCoreWindow, Event as IcedCoreEvent, Size,
};

use iced_runtime::{user_interface, Command, Debug, Program, UserInterface};

use iced_style::application::StyleSheet;

use iced_futures::{Executor, Runtime, Subscription};

use layershellev::{LayerEvent, ReturnData, WindowState, WindowWrapper};

use futures::{channel::mpsc, StreamExt};

use crate::{event::IcedLayerEvent, proxy::IcedProxy, settings::Settings};

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

// a dispatch loop, another is listen loop
pub fn run<A, E, C>(
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

    let ev: WindowState<()> = layershellev::WindowState::new(&application.namespace())
        .with_single(true)
        .with_use_display_handle(true)
        .with_option_size(settings.layer_settings.size)
        .with_layer(settings.layer_settings.layer)
        .with_anchor(settings.layer_settings.anchor)
        .with_exclusize_zone(settings.layer_settings.exclusize_zone)
        .build()
        .unwrap();

    let window = Arc::new(ev.gen_wrapper());
    let compositor = C::new(compositor_settings, window.clone())?;
    let mut renderer = compositor.create_renderer();

    for font in settings.fonts {
        use iced_core::text::Renderer;

        renderer.load_font(font);
    }

    let state = State::new(&application, &ev);

    let (mut event_sender, event_receiver) = mpsc::unbounded::<IcedLayerEvent>();

    let mut instance = Box::pin(run_instance::<A, E, C>(
        application,
        compositor,
        renderer,
        runtime,
        debug,
        event_receiver,
        state,
        init_command,
        window.clone(),
    ));

    let mut context = task::Context::from_waker(task::noop_waker_ref());

    #[allow(unused)]
    let _ = ev.running(move |event, ev, _| {
        use layershellev::DispatchMessage;
        match event {
            LayerEvent::InitRequest => {}
            // TODO: maybe use it later
            LayerEvent::BindProvide(_, _) => {}
            LayerEvent::RequestMessages(DispatchMessage::RequestRefresh { width, height }) => {
                event_sender
                    .start_send(IcedLayerEvent::RequestRefresh {
                        width: *width,
                        height: *height,
                    })
                    .expect("Cannot send");
            }
            _ => {}
        }
        let poll = instance.as_mut().poll(&mut context);
        if poll.is_ready() {
            ReturnData::RequestExist
        } else {
            layershellev::ReturnData::None
        }
    });
    Ok(())
}

#[allow(clippy::too_many_arguments)]
#[allow(unused)]
#[allow(unused_mut)]
async fn run_instance<A, E, C>(
    mut application: A,
    mut compositor: C,
    mut renderer: A::Renderer,
    mut runtime: Runtime<E, IcedProxy, A::Message>,
    mut debug: Debug,
    mut event_receiver: mpsc::UnboundedReceiver<IcedLayerEvent>,
    mut state: State<A>,
    init_command: Command<A::Message>,
    window: Arc<WindowWrapper>,
) where
    A: Application + 'static,
    E: Executor + 'static,
    C: Compositor<Renderer = A::Renderer> + 'static,
    A::Theme: StyleSheet,
{
    let physical_size = state.physical_size();
    let mut cache = user_interface::Cache::default();
    let mut surface =
        compositor.create_surface(window.clone(), physical_size.width, physical_size.height);
    let mut clipboard = LayerShellClipboard;

    let mut messages = Vec::new();

    // TODO: run command

    runtime.track(application.subscription().into_recipes());

    let mut user_interface = ManuallyDrop::new(build_user_interface(
        &application,
        cache,
        &mut renderer,
        state.logical_size(),
        &mut debug,
    ));

    debug.startup_finished();

    while let Some(event) = event_receiver.next().await {
        match event {
            IcedLayerEvent::RequestRefresh { width, height } => {
                state.update_view_port(width, height);
                debug.layout_started();
                user_interface =
                    ManuallyDrop::new(ManuallyDrop::into_inner(user_interface).relayout(
                        Size {
                            width: width as f32,
                            height: height as f32,
                        },
                        &mut renderer,
                    ));
                debug.layout_finished();

                compositor.configure_surface(&mut surface, width, height);
                let redraw_event = IcedCoreEvent::Window(
                    IcedCoreWindow::Id::MAIN,
                    IcedCoreWindow::Event::RedrawRequested(Instant::now()),
                );

                let (interface_state, _) = user_interface.update(
                    &[redraw_event.clone()],
                    IcedCoreMouse::Cursor::Unavailable,
                    &mut renderer,
                    &mut clipboard,
                    &mut messages,
                );
                // TODO: send event
                runtime.broadcast(redraw_event, iced_core::event::Status::Ignored);

                debug.draw_started();
                let new_mouse_interaction = user_interface.draw(
                    &mut renderer,
                    state.theme(),
                    &iced_core::renderer::Style {
                        text_color: state.text_color(),
                    },
                    state.cursor(),
                );
                debug.draw_finished();
                // TODO: check mosue_interaction

                debug.render_started();

                debug.draw_started();
                user_interface.draw(
                    &mut renderer,
                    &application.theme(),
                    &iced_core::renderer::Style {
                        text_color: state.text_color(),
                    },
                    IcedCoreMouse::Cursor::Unavailable,
                );
                debug.draw_finished();
                // TODO: draw mouse and something later
                compositor
                    .present(
                        &mut renderer,
                        &mut surface,
                        &state.viewport(),
                        state.background_color(),
                        &debug.overlay(),
                    )
                    .ok();
                debug.render_finished();
            }
        }
    }

    drop(ManuallyDrop::into_inner(user_interface));
}

/// Builds a [`UserInterface`] for the provided [`Application`], logging
/// [`struct@Debug`] information accordingly.
pub fn build_user_interface<'a, A: Application>(
    application: &'a A,
    cache: user_interface::Cache,
    renderer: &mut A::Renderer,
    size: Size,
    debug: &mut Debug,
) -> UserInterface<'a, A::Message, A::Theme, A::Renderer>
where
    A::Theme: StyleSheet,
{
    debug.view_started();
    let view = application.view();
    debug.view_finished();

    debug.layout_started();
    let user_interface = UserInterface::build(view, size, cache, renderer);
    debug.layout_finished();
    user_interface
}
