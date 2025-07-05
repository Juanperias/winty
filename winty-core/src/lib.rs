use std::ffi::c_void;

pub use winty_gl::GlHints;
pub use winty_gl::Profile;

pub trait Window {
    type Error;
    fn create(opts: WinOpts, hints: GlHints) -> Result<Self, Self::Error>
        where Self: Sized;
    fn toggle_fullscreen(&self) -> Result<(), Self::Error>;
    fn gl_swap_buffers(&self) -> Result<(), Self::Error>;
    fn gl_get_proc_address(&self, proc: &str) -> *const c_void;
    fn event_pump(&self) -> Result<impl EventPump, Self::Error>;
}

pub trait EventPump {}

#[derive(Debug)]
pub struct WinOpts {
    pub title: String,
    pub size: (u16, u16),
    pub pos: (i16, i16),
    pub fullscreen: bool,
    pub border_width: u16
}
