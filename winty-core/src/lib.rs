use std::ffi::c_void;

pub trait Window {
    type Error;
    fn create(opts: WinOpts) -> Result<Self, Self::Error>
        where Self: Sized;
    fn fullscreen(&self) -> Result<(), Self::Error>;
    fn gl_swap_buffers(&self) -> Result<(), Self::Error>;
    fn gl_get_proc_address(&self, proc: &str) -> *const c_void;
    fn show(&self) -> Result<impl EventPump, Self::Error>;
}

pub trait EventPump {}

#[derive(Debug)]
pub struct WinOpts {
    pub title: String,
    pub size: (u16, u16),
    pub pos: (u16, u16)
}
