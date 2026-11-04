use std::fmt::{Debug, Display, Formatter};
use std::sync::Arc;

use wayland_client::{Connection, Dispatch, QueueHandle, delegate_noop};
use wayland_protocols::xdg::foreign::zv2::client::{
    zxdg_exported_v2::{self, ZxdgExportedV2},
    zxdg_exporter_v2::ZxdgExporterV2,
    zxdg_imported_v2::{self, ZxdgImportedV2},
    zxdg_importer_v2::ZxdgImporterV2,
};

use crate::{DispatchMessage, ExWlGlobalShellState, QueuedMessage, id};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ForeignToplevelHandle(String);

impl ForeignToplevelHandle {
    /// Wrap a raw handle string, for example one passed on the command line or over D-Bus.
    pub fn new(handle: impl Into<String>) -> Self {
        Self(handle.into())
    }

    /// Parse the `parent_window` argument of an xdg-desktop-portal request.
    pub fn from_portal_parent_window(parent_window: &str) -> Option<Self> {
        let handle = parent_window.strip_prefix("wayland:")?;
        if handle.is_empty() {
            return None;
        }
        Some(Self(handle.to_owned()))
    }

    /// The raw handle string.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// The raw handle string.
    pub fn into_string(self) -> String {
        self.0
    }
}

impl From<ForeignToplevelHandle> for String {
    fn from(val: ForeignToplevelHandle) -> Self {
        val.0
    }
}

impl Display for ForeignToplevelHandle {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// The result of exporting a toplevel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ForeignHandleState {
    /// The compositor sent the handle.
    Ready(ForeignToplevelHandle),
    /// The compositor does not advertise `zxdg_exporter_v2`.
    Unsupported,
    /// The window does not exist.
    NoSurface,
    /// The window is not an xdg toplevel.
    NotToplevel,
    /// The export was revoked (closed before the handle arrived).
    Cancelled,
}

/// Receives the result of exporting a toplevel. See [ActivationTokenSink](crate::ActivationTokenSink).
#[derive(Clone)]
pub struct ForeignHandleSink(Option<Arc<dyn Fn(ForeignHandleState) + Send + Sync>>);

impl ForeignHandleSink {
    /// Receive the handle through events only, without a callback.
    pub fn none() -> Self {
        Self(None)
    }

    /// Call the callback once with the export result.
    pub fn callback<F>(deliver: F) -> Self
    where
        F: Fn(ForeignHandleState) + Send + Sync + 'static,
    {
        Self(Some(Arc::new(deliver)))
    }

    /// Deliver a result to the callback, if one is set.
    pub fn deliver(&self, state: ForeignHandleState) {
        if let Some(sink) = self.0.as_ref() {
            sink(state);
        }
    }
}

impl Debug for ForeignHandleSink {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str("ForeignHandleSink")
    }
}

/// The export of window.
#[derive(Debug)]
pub(crate) struct ToplevelExport {
    pub(crate) exported: ZxdgExportedV2,
    /// Sent once by the compositor.
    pub(crate) handle: Option<ForeignToplevelHandle>,
    /// Sinks waiting for `zxdg_exported_v2.handle`.
    pub(crate) waiting: Vec<ForeignHandleSink>,
}

impl ToplevelExport {
    /// Revoke the export, cancelling the sinks still waiting for the handle.
    pub(crate) fn destroy(self) {
        for sink in self.waiting {
            sink.deliver(ForeignHandleState::Cancelled);
        }
        self.exported.destroy();
    }
}

delegate_noop!(@<T> ExWlGlobalShellState<T>: ignore ZxdgImporterV2);
delegate_noop!(@<T> ExWlGlobalShellState<T>: ignore ZxdgExporterV2);

impl<T> Dispatch<ZxdgImportedV2, id::Id> for ExWlGlobalShellState<T> {
    fn event(
        state: &mut Self,
        proxy: &ZxdgImportedV2,
        event: zxdg_imported_v2::Event,
        id: &id::Id,
        _conn: &Connection,
        _qhandle: &QueueHandle<Self>,
    ) {
        let zxdg_imported_v2::Event::Destroyed = event else {
            return;
        };
        log::warn!(target: "exwlshellev", "foreign parent of {id:?} was destroyed by the compositor");
        let Some(unit) = state.shells.iter_mut().find(|unit| unit.id == *id) else {
            proxy.destroy();
            return;
        };
        if unit.foreign_parent.as_ref() != Some(proxy) {
            proxy.destroy();
            return;
        }
        if let Some(imported) = unit.foreign_parent.take() {
            imported.destroy();
        }
        state.messages.push(QueuedMessage::Targeted(
            *id,
            DispatchMessage::ForeignParentDestroyed,
        ));
    }
}

impl<T> Dispatch<ZxdgExportedV2, id::Id> for ExWlGlobalShellState<T> {
    fn event(
        state: &mut Self,
        proxy: &ZxdgExportedV2,
        event: zxdg_exported_v2::Event,
        id: &id::Id,
        _conn: &Connection,
        _qhandle: &QueueHandle<Self>,
    ) {
        let zxdg_exported_v2::Event::Handle { handle } = event else {
            return;
        };
        let Some(export) = state
            .shells
            .iter_mut()
            .find(|unit| unit.id == *id)
            .and_then(|unit| unit.export.as_mut())
            .filter(|export| export.exported == *proxy)
        else {
            return;
        };
        let handle = ForeignToplevelHandle::new(handle);
        for sink in export.waiting.drain(..) {
            sink.deliver(ForeignHandleState::Ready(handle.clone()));
        }
        export.handle = Some(handle.clone());
        state.messages.push(QueuedMessage::Targeted(
            *id,
            DispatchMessage::ToplevelExported(handle),
        ));
    }
}
