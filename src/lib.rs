use thiserror::Error;

#[cfg(not(any(feature = "x11")))]
compile_error!("You must have at least one feature activated");

#[derive(Error, Debug)]
pub enum WintyError {
    #[cfg(feature = "x11")]
    #[error("{0}")]
    X11Error(#[from] winty_x11::X11Error),


}


#[cfg(feature = "x11")]
pub mod x11 {
    pub use winty_x11::{X11Window, X11EventPump};    
}

pub mod window {
    pub use winty_core::{Window, Profile, GlHints, WinOpts};
    use winty_dpi::{PhysicalPosition, PhysicalSize};
    #[cfg(feature = "x11")]
    pub use winty_x11::X11Window;

    pub enum WintyWindow {
        #[cfg(feature = "x11")]
        X11(X11Window),
    }

    impl Window for WintyWindow {
        type Error = crate::WintyError;

        /// never call WintyBackend::create
        fn create(_opts: WinOpts, _hints: GlHints) -> Result<Self, Self::Error>
                where Self: Sized {
            panic!("never call WintyBackend::create");
        }

        fn gl_swap_buffers(&mut self) -> Result<(), Self::Error> {
            match self {
                #[cfg(feature = "x11")]
                Self::X11(x) => x.gl_swap_buffers()?,
               
                #[cfg(not(any(feature = "x11")))]
                _ => {
                    compile_error!("You must have at least one feature activated");
                    unreachable!();
                },

                #[allow(unreachable_patterns)]
                _ => {
                    panic!("Cannot create a window for your platform");
                },
            };
            Ok(())
        }
        fn event_pump(&mut self) -> Result<impl crate::event::EventPump, Self::Error> {
            let pump = match self {
                #[cfg(feature = "x11")]
                Self::X11(x) => x.event_pump(),
                
                #[cfg(not(any(feature = "x11")))]
                _ => {
                    compile_error!("You must have at least one feature activated");
                    unreachable!();
                },

                #[allow(unreachable_patterns)]
                 _ => {
                    panic!("Cannot create a window for your platform");
                },
            }?;
            
            Ok(pump)
        }
        fn gl_get_proc_address(&self, proc: &str) -> *const std::ffi::c_void {
            match self {
                #[cfg(feature = "x11")]
                Self::X11(x) => x.gl_get_proc_address(proc),

                #[cfg(not(any(feature = "x11")))]
                _ => {
                    compile_error!("You must have at least one feature activated");
                    unreachable!();
                },

                #[allow(unreachable_patterns)]
                _ => {
                    panic!("Cannot create a window for your platform");
                },
            }
        }
     
        fn toggle_fullscreen(&mut self) -> Result<(), Self::Error> {
            match self {
                #[cfg(feature = "x11")]
                Self::X11(x) => x.toggle_fullscreen()?,

                #[cfg(not(any(feature = "x11")))]
                _ => {
                     compile_error!("You must have at least one feature activated");
                     unreachable!();
                },

                #[allow(unreachable_patterns)]
                _ => {
                    panic!("Cannot create a window for your platform");
                },
            };

            Ok(())
        }
        fn scale_factor(&self) -> f64 {
            match self {
                Self::X11(x) => x.scale_factor(),

                #[cfg(not(any(feature = "x11")))]
                _ => {
                     compile_error!("You must have at least one feature activated");
                     unreachable!();
                },

                #[allow(unreachable_patterns)]
                _ => {
                    panic!("Cannot create a window for your platform");
                },
            }
        }
    }

    pub struct WindowBuilder {
        win_opts: crate::window::WinOpts,
        gl_hints: crate::window::GlHints,
    }

    impl WindowBuilder {
        pub fn create() -> Self {
            Self {
                win_opts: WinOpts {
                    title: "Winty Window".to_string(),
                    size: crate::dpi::Size::Physical(PhysicalSize::new(800, 600)),
                    pos: crate::dpi::Pos::Physical(PhysicalPosition::new(600, 600)),
                    fullscreen: false,
                    border_width: 10,
                },
                gl_hints: GlHints {
                    version: (3, 2),
                    profile: Profile::Core,
                }
            }
        }
        pub fn title<T: Into<String>>(mut self, title: T) -> Self {
            self.win_opts.title = title.into(); 
            self
        }
        pub fn size(mut self, size: crate::dpi::Size) -> Self {
            self.win_opts.size = size;
            self
        }
        pub fn pos(mut self, pos: crate::dpi::Pos) -> Self {
            self.win_opts.pos = pos;
            self
        }
        pub fn fullscreen(mut self, fullscreen: bool) -> Self {
            self.win_opts.fullscreen = fullscreen;
            self
        }
        pub fn gl_version(mut self, version: (u8, u8)) -> Self {
            self.gl_hints.version = version;
            self
        }
        pub fn profile(mut self, profile: Profile) -> Self {
            self.gl_hints.profile = profile;
            self
        }
        pub fn border_width(mut self, border_width: u16) -> Self {
            self.win_opts.border_width = border_width;
            self
        }
        pub fn build(&self) -> Result<WintyWindow, crate::WintyError> {
            if std::env::var("DISPLAY").is_ok() {
                #[cfg(feature = "x11")]
                return Ok(WintyWindow::X11(X11Window::create(self.win_opts.clone(), self.gl_hints)?));
            }

            panic!("Cannot create window for your platform");
        }
    }
}

pub mod dpi {
    pub use winty_dpi::{Pos, Size, LogicalPosition, LogicalSize, PhysicalPosition, PhysicalSize, Pixel};
}

pub mod key {
    pub use winty_core::key::Code;
}

pub mod event {
    pub use winty_core::{Event, EventPump};
}
