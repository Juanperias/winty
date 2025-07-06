use std::ffi::c_void;
use std::fmt::Debug;

pub use winty_gl::GlHints;
pub use winty_gl::Profile;
pub mod key;

pub trait Window {
    type Error;
    fn create(opts: WinOpts, hints: GlHints) -> Result<Self, Self::Error>
        where Self: Sized;
    fn toggle_fullscreen(&mut self) -> Result<(), Self::Error>;
    fn gl_swap_buffers(&mut self) -> Result<(), Self::Error>;
    fn gl_get_proc_address(&self, proc: &str) -> *const c_void;
    fn event_pump(&mut self) -> Result<impl EventPump, Self::Error>;
}

pub enum WName {
    X11,
}

pub trait EventPump {
    type Error: Debug;
    fn wait_for_event(&self) -> Result<Event, Self::Error>;
}

#[derive(Debug, Clone, Copy)]
pub enum Event {
    Redraw,
    Unknown,
    KeyPress(crate::key::Code)
    // TODO: put more events
}

#[derive(Debug, Clone)]
pub struct WinOpts {
    pub title: String,
    pub size: (u16, u16),
    pub pos: (i16, i16),
    pub fullscreen: bool,
    pub border_width: u16
}
