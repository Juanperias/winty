use winty::{event::{Event, EventPump}, window::{Window, WindowBuilder}, WintyError};  

fn main() -> Result<(), WintyError> {
    let mut win = WindowBuilder::create().title("Hello from winty!").build()?;

    gl::load_with(|proc| win.gl_get_proc_address(proc));

    unsafe {
        gl::Viewport(0, 0, 800, 600);
    }

    loop {
        unsafe {
            gl::ClearColor(1.0, 0.0, 0.0, 1.0);
            gl::Clear(gl::COLOR_BUFFER_BIT);
        }

        win.gl_swap_buffers()?;

        {
            let pump = win.event_pump()?;

            match pump.wait_for_event().unwrap() { 
                Event::KeyPress(winty::key::Code::Escape) => {
                    println!("Bye!");
                    break;
                },
                _ => {},
            }
        }

        win.gl_swap_buffers()?;

    }

    Ok(())
}
