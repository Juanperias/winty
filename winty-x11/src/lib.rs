use std::{ffi::{c_void, CString}, num::NonZeroU32, ptr::NonNull};

use raw_window_handle::{RawDisplayHandle, RawWindowHandle, XcbDisplayHandle, XcbWindowHandle};
use winty_core::{EventPump, WinOpts, Window};
use thiserror::Error;
use winty_gl::{GlHandler, GlHandlerError, GlHints};
use xcb::{x, Connection, Xid};

xcb::atoms_struct! {
    #[derive(Debug)]
    pub struct Atoms {
        wm_protocols    => b"WM_PROTOCOLS",
        wm_del_window   => b"WM_DELETE_WINDOW",
        wm_state        => b"_NET_WM_STATE",
        wm_state_maxv   => b"_NET_WM_STATE_MAXIMIZED_VERT",
        wm_state_maxh   => b"_NET_WM_STATE_MAXIMIZED_HORZ",
        wm_fullscreen   => b"_NET_WM_STATE_FULLSCREEN", 
    }
}

#[derive(Error, Debug)]
pub enum X11Error {
    #[error("Connection Error {0}")]
    ConnError(#[from] xcb::ConnError),

    #[error("Gl Error {0}")]
    GlError(#[from] GlHandlerError),

    #[error("Xcb Error {0}")]
    XcbError(#[from] xcb::Error),

    #[error("Protocol Error {0}")]
    ProtocolError(#[from] xcb::ProtocolError),
}

pub struct X11Window {
    conn: Connection,
    window: x::Window,
    gl_context: GlHandler,
    opts: WinOpts,
    atoms: Atoms
}

impl Window for X11Window {
    type Error = X11Error;
    fn create(opts: WinOpts, hints: GlHints) -> Result<Self, Self::Error>
            where Self: Sized {
        let (conn, screen_num) = xcb::Connection::connect(None)?;
        let setup = conn.get_setup();
        let raw_conn = NonNull::new(conn.get_raw_conn() as *mut c_void).unwrap();

        let screen = setup.roots().nth(screen_num as usize).unwrap();
        let window: x::Window = conn.generate_id();

            
        conn.send_request(&x::CreateWindow {
            depth: x::COPY_FROM_PARENT as u8,
            wid: window,
            parent: screen.root(),
            x: opts.pos.0,
            y: opts.pos.1,
            width: opts.size.0,
            height: opts.size.1,
            border_width: opts.border_width,
            class: x::WindowClass::InputOutput,
            visual: screen.root_visual(),
            value_list: &[
                x::Cw::EventMask(x::EventMask::EXPOSURE | x::EventMask::KEY_PRESS)
            ],
        });

        conn.send_request(&x::ChangeProperty {
            mode: x::PropMode::Replace,
            window,
            property: x::ATOM_WM_NAME,
            r#type: x::ATOM_STRING,
            data: opts.title.as_bytes()
        });

        conn.flush()?;

        let cookie = conn.send_request(&x::GetProperty {
            delete: false,
            window,
            property: x::ATOM_WM_NAME,
            r#type: x::ATOM_STRING,
            long_offset: 0,
            long_length: 1024,
        });

        let reply = conn.wait_for_reply(cookie)?;
        assert_eq!(reply.value::<u8>(), opts.title.as_bytes());

        let atoms = Atoms::intern_all(&conn)?;

        conn.send_and_check_request(&x::ChangeProperty {
            mode: x::PropMode::Replace,
            window,
            property: atoms.wm_protocols,
            r#type: x::ATOM_ATOM,
            data: &[atoms.wm_del_window],
        })?;

        if opts.fullscreen {
            conn.send_and_check_request(&x::ChangeProperty {
                mode: x::PropMode::Replace,
                window,
                property: atoms.wm_state,
                r#type: x::ATOM_ATOM,
                data: &[atoms.wm_fullscreen],
            })?;
        }

        conn.send_request(&x::MapWindow {
            window
        });

        let display_handle = RawDisplayHandle::Xcb(XcbDisplayHandle::new(Some(raw_conn), screen_num));
        let window_handle = RawWindowHandle::Xcb(XcbWindowHandle::new(NonZeroU32::new(window.resource_id()).unwrap()));

        let gl_handle = GlHandler::new(display_handle, window_handle, hints, (opts.size.0 as u32, opts.size.1 as u32))?;

        Ok(Self {
            gl_context: gl_handle,
            conn,
            window,
            opts,
            atoms
        })
    
    }
    fn toggle_fullscreen(&self) -> Result<(), Self::Error> {
        let mut data = Vec::new();
        if !self.opts.fullscreen {
            data.push(self.atoms.wm_fullscreen);
        }

        self.conn.send_and_check_request(&x::ChangeProperty {
            mode: x::PropMode::Replace,
            window: self.window,
            property: self.atoms.wm_state,
            r#type: x::ATOM_ATOM,
            data: &data
        })?;
        

        Ok(())
    }
    fn gl_swap_buffers(&self) -> Result<(), Self::Error> {
        self.gl_context.swap_buffers()?;
        Ok(())
    }
    fn gl_get_proc_address(&self, proc: &str) -> *const c_void {
        let cstring = CString::new(proc).unwrap();
        self.gl_context.get_proc_address(cstring.as_c_str())
    }
    fn event_pump(&self) -> Result<impl EventPump, Self::Error> {
        Ok(X11EventPump)        
    }
}

pub struct X11EventPump;

impl EventPump for X11EventPump {}
