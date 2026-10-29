//! Native window integration using `winit` + `softbuffer`.
//!
//! Re-runs block layout, display-list generation, and software rasterization
//! whenever the window is resized or requests a redraw, so `width: auto` fluid
//! containers and `margin: auto` centered blocks respond live to window sizing.

use super::rasterizer::{paint, Canvas};
use crate::layout::layout_document;
use crate::style::StyledDocument;
use crate::util::Color;
use std::error::Error;
use std::fmt;
use std::num::NonZeroU32;
use std::rc::Rc;
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::keyboard::{Key, NamedKey};
use winit::window::{Window, WindowId};

/// Configuration for the browser preview window.
#[derive(Debug, Clone)]
pub struct WindowConfig {
    pub title: String,
    pub width: u32,
    pub height: u32,
    pub background: Color,
}

impl Default for WindowConfig {
    fn default() -> Self {
        Self {
            title: "Sayanox Core".to_owned(),
            width: 800,
            height: 600,
            background: Color::WHITE,
        }
    }
}

#[derive(Debug)]
pub enum WindowError {
    EventLoop(winit::error::EventLoopError),
    WindowCreation(winit::error::OsError),
    SoftBuffer(softbuffer::SoftBufferError),
}

impl fmt::Display for WindowError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EventLoop(e) => write!(f, "window event loop error: {e}"),
            Self::WindowCreation(e) => write!(f, "failed to create window: {e}"),
            Self::SoftBuffer(e) => write!(f, "framebuffer error: {e}"),
        }
    }
}

impl Error for WindowError {}

impl From<winit::error::EventLoopError> for WindowError {
    fn from(err: winit::error::EventLoopError) -> Self {
        Self::EventLoop(err)
    }
}

/// Open a native window and display `document` using the software rasterizer.
pub fn open_window(document: &StyledDocument<'_>, config: WindowConfig) -> Result<(), WindowError> {
    let event_loop = EventLoop::new()?;
    let mut app = BrowserApp {
        document,
        config,
        state: None,
        canvas: Canvas::new(0, 0, Color::WHITE),
        init_error: None,
    };
    event_loop.run_app(&mut app)?;
    if let Some(err) = app.init_error {
        return Err(err);
    }
    Ok(())
}

struct SurfaceState {
    window: Rc<Window>,
    _context: softbuffer::Context<Rc<Window>>,
    surface: softbuffer::Surface<Rc<Window>, Rc<Window>>,
}

struct BrowserApp<'doc, 'dom> {
    document: &'doc StyledDocument<'dom>,
    config: WindowConfig,
    state: Option<SurfaceState>,
    canvas: Canvas,
    init_error: Option<WindowError>,
}

impl BrowserApp<'_, '_> {
    fn render_frame(&mut self) -> Result<(), softbuffer::SoftBufferError> {
        let Some(state) = &mut self.state else {
            return Ok(());
        };

        let size = state.window.inner_size();
        let (Some(width), Some(height)) = (NonZeroU32::new(size.width), NonZeroU32::new(size.height))
        else {
            // Minimized windows can report a 0x0 surface; skip presenting until restored.
            return Ok(());
        };

        state.surface.resize(width, height)?;

        let w = width.get() as usize;
        let h = height.get() as usize;
        let layout_root = layout_document(self.document, w as f32, h as f32);
        self.canvas = paint(&layout_root, w, h, self.config.background);

        let mut buffer = state.surface.buffer_mut()?;
        self.canvas.copy_to_0rgb_buffer(&mut buffer);
        buffer.present()?;
        Ok(())
    }
}

impl ApplicationHandler for BrowserApp<'_, '_> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.state.is_some() {
            return;
        }

        let attrs = Window::default_attributes()
            .with_title(self.config.title.clone())
            .with_inner_size(LogicalSize::new(self.config.width, self.config.height));

        let window = match event_loop.create_window(attrs) {
            Ok(w) => Rc::new(w),
            Err(err) => {
                self.init_error = Some(WindowError::WindowCreation(err));
                event_loop.exit();
                return;
            }
        };

        let context = match softbuffer::Context::new(window.clone()) {
            Ok(ctx) => ctx,
            Err(err) => {
                self.init_error = Some(WindowError::SoftBuffer(err));
                event_loop.exit();
                return;
            }
        };

        let surface = match softbuffer::Surface::new(&context, window.clone()) {
            Ok(surf) => surf,
            Err(err) => {
                self.init_error = Some(WindowError::SoftBuffer(err));
                event_loop.exit();
                return;
            }
        };

        window.request_redraw();
        self.state = Some(SurfaceState {
            window,
            _context: context,
            surface,
        });
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        let Some(current_id) = self.state.as_ref().map(|s| s.window.id()) else {
            return;
        };
        if window_id != current_id {
            return;
        }

        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::KeyboardInput { event, .. }
                if event.state.is_pressed()
                    && matches!(event.logical_key, Key::Named(NamedKey::Escape)) =>
            {
                event_loop.exit();
            }
            WindowEvent::Resized(_) => {
                if let Some(state) = &self.state {
                    state.window.request_redraw();
                }
            }
            WindowEvent::RedrawRequested => {
                if let Err(err) = self.render_frame() {
                    self.init_error = Some(WindowError::SoftBuffer(err));
                    event_loop.exit();
                }
            }
            _ => {}
        }
    }
}
