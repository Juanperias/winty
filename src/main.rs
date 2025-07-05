use winty_core::Window;
use winty_x11::X11Window;

fn main() {
 
    let window = X11Window::create(winty_core::WinOpts { title: "Hey!".to_string(), size: (200, 200), pos: (0, 0), fullscreen: false, border_width: 20 }, winty_core::GlHints { version: (3, 2), profile: winty_core::Profile::Core  }).unwrap();
    
}
