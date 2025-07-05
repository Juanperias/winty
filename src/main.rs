use std::ffi::c_uint;

use winty_core::{Event, EventPump, Window};
use winty_x11::X11Window;

type GLint =        i32;
type GLsizei =      i32;
type GLuint =       c_uint;
type GLfloat =      f32;


#[link(kind = "dylib", name = "GL")]
unsafe extern "C" {
    fn glViewport(x: GLint, y: GLint, width: GLsizei, height: GLsizei) -> ();
    fn glClear(bit: GLuint);
    fn glClearColor(red: GLfloat, green: GLfloat, blue: GLfloat, alpha: GLfloat) -> ();
}


fn main() {
    let window = X11Window::create(winty_core::WinOpts { title: "Winty Window".to_string(), size: (200, 200), pos: (0, 0), fullscreen: true, border_width: 20 }, winty_core::GlHints { version: (3, 2), profile: winty_core::Profile::Core  }).unwrap();
    
    unsafe { glViewport(0, 0, 200, 200); }
        
    loop {
        match window.event_pump().unwrap().wait_for_event().unwrap() {
            Event::Redraw => {
                unsafe {
                    glClearColor(1.0, 0.5, 0.0, 1.0);
                    glClear(0x00004000);
                }
                window.gl_swap_buffers().unwrap();
            },
            _ => {},
        }
    }
}
