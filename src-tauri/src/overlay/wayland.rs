//! The card as a layer-shell surface: overlay layer, top-left corner, no keyboard focus and no
//! input region, so the game keeps its controls and clicks go through.

use super::draw::{self, CARD_H, PAD, SURFACE_W};
use super::Popup;
use smithay_client_toolkit::{
    compositor::{CompositorHandler, CompositorState, FrameCallbackData, Region},
    delegate_registry,
    output::{OutputHandler, OutputState},
    registry::{ProvidesRegistryState, RegistryState},
    registry_handlers,
    shell::{
        wlr_layer::{Anchor, KeyboardInteractivity, Layer, LayerShell, LayerShellHandler, LayerSurface, LayerSurfaceConfigure},
        WaylandSurface,
    },
    shm::{slot::SlotPool, Shm, ShmHandler},
};
use std::time::{Duration, Instant};
use wayland_client::{
    globals::registry_queue_init,
    protocol::{wl_output, wl_shm, wl_surface},
    Connection, QueueHandle,
};

const SLIDE: Duration = Duration::from_millis(450);

struct Overlay {
    registry_state: RegistryState,
    output_state: OutputState,
    shm: Shm,
    pool: SlotPool,
    layer: Option<LayerSurface>,
    popup: Popup,
    scale: i32,
    configured: bool,
    started: Option<Instant>,
    exit: bool,
}

/// Shows the card until its time is up, on the output named `output` (for example "DP-2")
/// or, without one, where the compositor puts it (the focused output).
pub fn show(popup: &Popup, output: Option<&str>) -> Result<(), String> {
    let conn = Connection::connect_to_env().map_err(|e| format!("no Wayland session: {e}"))?;
    let (globals, mut queue) = registry_queue_init(&conn).map_err(|e| e.to_string())?;
    let qh = queue.handle();
    let compositor = CompositorState::bind(&globals, &qh).map_err(|e| e.to_string())?;
    let layer_shell = LayerShell::bind(&globals, &qh).map_err(|_| "this compositor has no layer-shell (GNOME or X11)".to_string())?;
    let shm = Shm::bind(&globals, &qh).map_err(|e| e.to_string())?;

    let size = (SURFACE_W * 3.0) as usize * ((CARD_H + PAD * 2.0) * 3.0) as usize * 4;
    let pool = SlotPool::new(size, &shm).map_err(|e| e.to_string())?;
    let mut overlay = Overlay {
        registry_state: RegistryState::new(&globals),
        output_state: OutputState::new(&globals, &qh),
        shm,
        pool,
        layer: None,
        popup: popup.clone(),
        scale: 1,
        configured: false,
        started: None,
        exit: false,
    };
    // Outputs are announced after the first round trips.
    queue.roundtrip(&mut overlay).map_err(|e| e.to_string())?;
    queue.roundtrip(&mut overlay).map_err(|e| e.to_string())?;
    let target = output.and_then(|name| {
        overlay.output_state.outputs().find(|o| overlay.output_state.info(o).and_then(|i| i.name).as_deref() == Some(name))
    });

    let surface = compositor.create_surface(&qh);
    // No input region: clicks and the pointer go to the game underneath.
    if let Ok(region) = Region::new(&compositor) {
        surface.set_input_region(Some(region.wl_region()));
    }
    let layer = layer_shell.create_layer_surface(&qh, surface, Layer::Overlay, Some("portshelf-achievement"), target.as_ref());
    layer.set_anchor(Anchor::TOP | Anchor::LEFT);
    layer.set_margin(24 - PAD as i32, 0, 0, 0);
    layer.set_keyboard_interactivity(KeyboardInteractivity::None);
    layer.set_exclusive_zone(-1);
    layer.set_size(SURFACE_W as u32, (CARD_H + PAD * 2.0) as u32);
    layer.commit();
    overlay.layer = Some(layer);
    while !overlay.exit {
        queue.blocking_dispatch(&mut overlay).map_err(|e| e.to_string())?;
    }
    Ok(())
}

impl Overlay {
    /// Progress of the slide in and out, or None once the card is done.
    fn progress(&mut self) -> Option<f32> {
        let started = *self.started.get_or_insert_with(Instant::now);
        let t = started.elapsed();
        let hold = Duration::from_secs_f32(self.popup.seconds.max(1.0));
        if t < SLIDE {
            Some(t.as_secs_f32() / SLIDE.as_secs_f32())
        } else if t < SLIDE + hold {
            Some(1.0)
        } else if t < SLIDE * 2 + hold {
            Some(1.0 - (t - SLIDE - hold).as_secs_f32() / SLIDE.as_secs_f32())
        } else {
            None
        }
    }

    fn draw(&mut self, qh: &QueueHandle<Self>) {
        let Some(progress) = self.progress() else {
            self.exit = true;
            return;
        };
        let pixmap = draw::render(&self.popup, self.scale as f32, progress);
        let (w, h) = (pixmap.width() as i32, pixmap.height() as i32);
        let Ok((buffer, canvas)) = self.pool.create_buffer(w, h, w * 4, wl_shm::Format::Argb8888) else {
            self.exit = true;
            return;
        };
        // tiny-skia is premultiplied RGBA; wl_shm ARGB8888 is premultiplied BGRA in memory.
        for (dst, src) in canvas.chunks_exact_mut(4).zip(pixmap.data().chunks_exact(4)) {
            dst.copy_from_slice(&[src[2], src[1], src[0], src[3]]);
        }
        let Some(layer) = &self.layer else { return };
        let surface = layer.wl_surface();
        surface.set_buffer_scale(self.scale);
        surface.damage_buffer(0, 0, w, h);
        surface.frame(qh, FrameCallbackData(surface.clone()));
        let _ = buffer.attach_to(surface);
        layer.commit();
    }
}

impl CompositorHandler for Overlay {
    fn scale_factor_changed(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &wl_surface::WlSurface, factor: i32) {
        self.scale = factor.max(1);
    }
    fn transform_changed(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &wl_surface::WlSurface, _: wl_output::Transform) {}
    fn frame(&mut self, _: &Connection, qh: &QueueHandle<Self>, _: &wl_surface::WlSurface, _: u32) {
        self.draw(qh);
    }
    fn surface_enter(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &wl_surface::WlSurface, _: &wl_output::WlOutput) {}
    fn surface_leave(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &wl_surface::WlSurface, _: &wl_output::WlOutput) {}
}

impl OutputHandler for Overlay {
    fn output_state(&mut self) -> &mut OutputState {
        &mut self.output_state
    }
    fn new_output(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_output::WlOutput) {}
    fn update_output(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_output::WlOutput) {}
    fn output_destroyed(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_output::WlOutput) {}
}

impl LayerShellHandler for Overlay {
    fn closed(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &LayerSurface) {
        self.exit = true;
    }
    fn configure(&mut self, _: &Connection, qh: &QueueHandle<Self>, _: &LayerSurface, _: LayerSurfaceConfigure, _: u32) {
        if !self.configured {
            self.configured = true;
            self.draw(qh);
        }
    }
}

impl ShmHandler for Overlay {
    fn shm_state(&mut self) -> &mut Shm {
        &mut self.shm
    }
}

delegate_registry!(Overlay);

impl ProvidesRegistryState for Overlay {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry_state
    }
    registry_handlers![OutputState];
}

smithay_client_toolkit::delegate_dispatch2!(Overlay);
