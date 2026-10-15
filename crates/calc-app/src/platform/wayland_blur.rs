//! Wayland: blur behind the window through `ext-background-effect-v1`, which KWin 6.5+
//! offers in place of `org_kde_kwin_blur` (the only protocol winit 0.30 knows). The
//! requests go over winit's own connection, on a separate event queue.

#![allow(unsafe_code)]

use std::ffi::c_void;
use std::ptr::NonNull;
use wayland_backend::client::{Backend, ObjectId};
use wayland_client::globals::{GlobalListContents, registry_queue_init};
use wayland_client::protocol::wl_compositor::WlCompositor;
use wayland_client::protocol::wl_region::WlRegion;
use wayland_client::protocol::wl_registry::WlRegistry;
use wayland_client::protocol::wl_surface::WlSurface;
use wayland_client::{Connection, Dispatch, EventQueue, Proxy, QueueHandle, delegate_noop};
use wayland_protocols::ext::background_effect::v1::client::ext_background_effect_manager_v1::ExtBackgroundEffectManagerV1;
use wayland_protocols::ext::background_effect::v1::client::ext_background_effect_surface_v1::ExtBackgroundEffectSurfaceV1;

/// The window's background-effect object. The protocol allows only one per surface,
/// so it is created once and kept for the window's lifetime.
pub struct WaylandBlur {
    conn: Connection,
    queue: EventQueue<State>,
    compositor: WlCompositor,
    effect: ExtBackgroundEffectSurfaceV1,
}

struct State;

impl Dispatch<WlRegistry, GlobalListContents> for State {
    fn event(
        _: &mut State,
        _: &WlRegistry,
        _: <WlRegistry as Proxy>::Event,
        _: &GlobalListContents,
        _: &Connection,
        _: &QueueHandle<State>,
    ) {
    }
}

delegate_noop!(State: WlCompositor);
delegate_noop!(State: WlRegion);
delegate_noop!(State: ignore ExtBackgroundEffectManagerV1);
delegate_noop!(State: ExtBackgroundEffectSurfaceV1);

impl WaylandBlur {
    /// `None` when the compositor doesn't offer the protocol.
    ///
    /// # Safety
    /// `display` and `surface` must be winit's live `wl_display` and the window's
    /// `wl_surface`, and both must outlive the returned value.
    pub unsafe fn new(display: NonNull<c_void>, surface: NonNull<c_void>) -> Option<WaylandBlur> {
        // SAFETY: the caller guarantees a live display; a foreign display is not closed
        // when this connection is dropped.
        let backend = unsafe { Backend::from_foreign_display(display.as_ptr().cast()) };
        let conn = Connection::from_backend(backend);
        let (globals, queue) = registry_queue_init::<State>(&conn).ok()?;
        let qh = queue.handle();
        let manager: ExtBackgroundEffectManagerV1 = globals.bind(&qh, 1..=1, ()).ok()?;
        let compositor: WlCompositor = globals.bind(&qh, 1..=4, ()).ok()?;
        // SAFETY: the caller guarantees a live wl_surface proxy on this display.
        let id =
            unsafe { ObjectId::from_ptr(WlSurface::interface(), surface.as_ptr().cast()) }.ok()?;
        let surface = WlSurface::from_id(&conn, id).ok()?;
        let effect = manager.get_background_effect(&surface, &qh, ());
        Some(WaylandBlur {
            conn,
            queue,
            compositor,
            effect,
        })
    }

    /// Blurs behind the whole window, or nothing. Applied with the surface's next
    /// commit, which the renderer makes on the next frame.
    pub fn set(&mut self, on: bool) {
        let qh = self.queue.handle();
        if on {
            let region = self.compositor.create_region(&qh, ());
            region.add(0, 0, i32::MAX, i32::MAX);
            self.effect.set_blur_region(Some(&region));
            region.destroy();
        } else {
            self.effect.set_blur_region(None);
        }
        if let Err(e) = self.conn.flush() {
            tracing::debug!("cannot send the blur region: {e}");
        }
        let _ = self.queue.dispatch_pending(&mut State);
    }
}
