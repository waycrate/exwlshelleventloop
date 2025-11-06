use std::{collections::HashMap, mem};

use iced::{Event, Program, Size, event::Status, mouse::Cursor, window::Id};
use iced_core::{Clipboard, renderer::Style, widget::Operation};
use iced_program::Instance;
use iced_runtime::{
    UserInterface as IcedUserInterface,
    user_interface::{Cache, State},
};

use iced_debug as debug;

pub(crate) trait UserInterfaceReclaim<Message, Theme, Renderer> {
    fn reclaim(&mut self, ui: IcedUserInterface<'static, Message, Theme, Renderer>);
}

/// Provide a guard to hold the ui and prevent leaking the reference to the application. A user can hold this guard without querying from map each time.
/// When this guard is dropped, it will return the ui to the manager if it is not taken.
pub(crate) struct UserInterfaceMutGuard<'a, Message, Theme, Renderer, Reclaim>
where
    Reclaim: UserInterfaceReclaim<Message, Theme, Renderer>,
{
    reclaim: Reclaim,
    /// Building 'static IcedUserInterface will draw nothing, so we should use the safe lifetime as
    /// application.
    ui: Option<IcedUserInterface<'a, Message, Theme, Renderer>>,
}

impl<'a, Message, Theme, Renderer, Reclaim>
    UserInterfaceMutGuard<'a, Message, Theme, Renderer, Reclaim>
where
    Renderer: iced_core::Renderer,
    Reclaim: UserInterfaceReclaim<Message, Theme, Renderer>,
{
    fn take(&mut self) -> IcedUserInterface<'a, Message, Theme, Renderer> {
        self.ui.take().expect("ui is taken")
    }

    pub fn draw(&mut self, renderer: &mut Renderer, theme: &Theme, style: &Style, cursor: Cursor) {
        let mut ui = self.take();
        ui.draw(renderer, theme, style, cursor);
        self.ui = Some(ui);
    }

    #[allow(unused)]
    pub fn into_cache(mut self) -> Cache {
        self.take().into_cache()
    }

    pub fn operate(&mut self, renderer: &Renderer, operation: &mut dyn Operation<()>) {
        let mut ui = self.take();
        ui.operate(renderer, operation);
        self.ui = Some(ui);
    }

    pub fn relayout(mut self, bounds: Size, renderer: &mut Renderer) -> Self {
        let ui = self.take().relayout(bounds, renderer);
        self.ui = Some(ui);
        self
    }

    pub fn update(
        &mut self,
        events: &[Event],
        cursor: Cursor,
        renderer: &mut Renderer,
        clipboard: &mut dyn Clipboard,
        messages: &mut Vec<Message>,
    ) -> (State, Vec<Status>) {
        let mut ui = self.take();
        let res = ui.update(events, cursor, renderer, clipboard, messages);
        self.ui = Some(ui);
        res
    }
}

impl<Message, Theme, Renderer, Reclaim> Drop
    for UserInterfaceMutGuard<'_, Message, Theme, Renderer, Reclaim>
where
    Reclaim: UserInterfaceReclaim<Message, Theme, Renderer>,
{
    fn drop(&mut self) {
        if let Some(ui) = self.ui.take() {
            // SAFETY There is no public api to change ui. It always refers to application.
            let ui: IcedUserInterface<'static, _, _, _> = unsafe { mem::transmute(ui) };
            self.reclaim.reclaim(ui);
        }
    }
}

pub struct UserInterfaces<P: Program> {
    // SAFETY application will only be dropped after all uis are dropped. And we won't
    // allow publicly access to IcedUserInterface<'static, A::Message, A::Theme, A::Renderer>, so
    // reference to application won't be leaked to public.
    #[allow(clippy::type_complexity)]
    uis: HashMap<Id, IcedUserInterface<'static, P::Message, P::Theme, P::Renderer>>,
    application: Instance<P>,
}

impl<P: Program> UserInterfaces<P>
where
    P: Program + 'static,
{
    pub fn new(application: Instance<P>) -> Self {
        Self {
            uis: HashMap::new(),
            application,
        }
    }

    pub fn application(&self) -> &Instance<P> {
        &self.application
    }

    pub fn remove(&mut self, id: &Id) -> Option<Cache> {
        self.uis.remove(id).map(IcedUserInterface::into_cache)
    }

    pub fn extract_all(&mut self) -> (HashMap<Id, Cache>, &mut Instance<P>) {
        // SAFETY remove all references before return mut reference of application
        let caches = self
            .uis
            .drain()
            .map(|(id, ui)| (id, ui.into_cache()))
            .collect();
        (caches, &mut self.application)
    }

    #[allow(clippy::type_complexity)]
    pub fn ui_mut(
        &mut self,
        id: &Id,
    ) -> Option<UserInterfaceMutGuard<'static, P::Message, P::Theme, P::Renderer, (&mut Self, Id)>>
    {
        self.uis.remove(id).map(|ui| UserInterfaceMutGuard {
            reclaim: (self, *id),
            ui: Some(ui),
        })
    }

    pub fn build(&mut self, id: Id, cache: Cache, renderer: &mut P::Renderer, size: Size) {
        let view_span = debug::view(id);
        let view = self.application.view(id);
        view_span.finish();

        let layout_span = debug::layout(id);
        let ui = IcedUserInterface::build(view, size, cache, renderer);
        layout_span.finish();
        // SAFETY ui won't outlive application.
        let ui: IcedUserInterface<'static, _, _, _> = unsafe { mem::transmute(ui) };
        self.uis.insert(id, ui);
    }
}

impl<P: Program> Drop for UserInterfaces<P> {
    fn drop(&mut self) {
        // SAFETY drop all references of application before dropping application
        self.uis.clear();
    }
}

impl<P: Program> UserInterfaceReclaim<P::Message, P::Theme, P::Renderer>
    for (&mut UserInterfaces<P>, Id)
{
    fn reclaim(&mut self, ui: IcedUserInterface<'static, P::Message, P::Theme, P::Renderer>) {
        self.0.uis.insert(self.1, ui);
    }
}
