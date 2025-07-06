use std::{ffi::{c_void, CString}, num::NonZeroU32, ptr::NonNull, sync::Arc};

use as_raw_xcb_connection::AsRawXcbConnection;
use raw_window_handle::{RawDisplayHandle, RawWindowHandle, XcbDisplayHandle, XcbWindowHandle};
use winty_core::{Event, EventPump, WinOpts, key, Window};
use thiserror::Error;
use winty_gl::{GlHandler, GlHandlerError, GlHints};
use xcb::{x, xkb, Connection, Xid};
use xkbcommon::xkb::Keymap;

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

// TODO: separate keyboard logic
pub struct X11Window {
    conn: Arc<Connection>,
    window: x::Window,
    gl_context: GlHandler,
    opts: WinOpts,
    atoms: Atoms,
    keycode_table: [key::Code; 256]
}

struct RawXcbWrapper(Arc<xcb::Connection>);

unsafe impl AsRawXcbConnection for RawXcbWrapper {
    fn as_raw_xcb_connection(&self) -> *mut as_raw_xcb_connection::xcb_connection_t {
        self.0.get_raw_conn() as *mut as_raw_xcb_connection::xcb_connection_t
    }
}

impl Window for X11Window {
    type Error = X11Error;
    fn create(opts: WinOpts, hints: GlHints) -> Result<Self, Self::Error>
            where Self: Sized {
        let (conn, screen_num) = xcb::Connection::connect_with_extensions(None, &[xcb::Extension::Xkb], &[])?;
        let conn = Arc::new(conn);
        
        {
            let xkb_ver = conn.wait_for_reply(conn.send_request(&xkb::UseExtension {
                wanted_major: 1,
                wanted_minor: 0,
            }))?;

            assert!(xkb_ver.supported(), "xkb-1.0 support is required!");
        }

        let setup = conn.get_setup();
        let raw_conn = NonNull::new(conn.get_raw_conn() as *mut c_void).unwrap();

        let screen = setup.roots().nth(screen_num as usize).unwrap();
        let window: x::Window = conn.generate_id();

        {
        let events = xkb::EventType::NEW_KEYBOARD_NOTIFY
            | xkb::EventType::MAP_NOTIFY
            | xkb::EventType::STATE_NOTIFY;

        let map_parts = xkb::MapPart::KEY_TYPES
            | xkb::MapPart::KEY_SYMS
            | xkb::MapPart::MODIFIER_MAP
            | xkb::MapPart::EXPLICIT_COMPONENTS
            | xkb::MapPart::KEY_ACTIONS
            | xkb::MapPart::KEY_BEHAVIORS
            | xkb::MapPart::VIRTUAL_MODS
            | xkb::MapPart::VIRTUAL_MOD_MAP;

        let cookie = conn.send_request_checked(&xkb::SelectEvents {
            device_spec: xkb::Id::UseCoreKbd as xkb::DeviceSpec,
            affect_which: events,
            clear: xkb::EventType::empty(),
            select_all: events,
            affect_map: map_parts,
            map: map_parts,
            details: &[],
        });

        conn.check_request(cookie)?;
    }

        let raw_xcb_wrapper = RawXcbWrapper(Arc::clone(&conn));
        let context = xkbcommon::xkb::Context::new(xkbcommon::xkb::COMPILE_NO_FLAGS);
        let device_id = xkbcommon::xkb::x11::get_core_keyboard_device_id(&raw_xcb_wrapper);
        let keymap = xkbcommon::xkb::x11::keymap_new_from_device(&context, &raw_xcb_wrapper, device_id, xkbcommon::xkb::KEYMAP_COMPILE_NO_FLAGS);

        
            
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
        } else {
            conn.send_and_check_request(&x::ChangeProperty {
                mode: x::PropMode::Replace,
                window,
                property: atoms.wm_state,
                r#type: x::ATOM_ATOM,
                data: &[0_u8],
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
            keycode_table: build_keycode_table(),
            atoms
        })
    
    }
    fn toggle_fullscreen(&mut self) -> Result<(), Self::Error> {
         if !self.opts.fullscreen {
            self.opts.fullscreen = true;
                    self.conn.send_and_check_request(&x::ChangeProperty {
            mode: x::PropMode::Replace,
            window: self.window,
            property: self.atoms.wm_state,
            r#type: x::ATOM_ATOM,
            data: &[self.atoms.wm_fullscreen]
        })?;

            return Ok(());
        }

        self.conn.send_and_check_request(&x::ChangeProperty {
            mode: x::PropMode::Replace,
            window: self.window,
            property: self.atoms.wm_state,
            r#type: x::ATOM_ATOM,
            data: &[0 as u8]
        })?;
      
        self.opts.fullscreen = false;
        Ok(())
    }
    fn gl_swap_buffers(&mut self) -> Result<(), Self::Error> {
        self.gl_context.swap_buffers()?;
        Ok(())
    }
    fn gl_get_proc_address(&self, proc: &str) -> *const c_void {
        let cstring = CString::new(proc).unwrap();
        self.gl_context.get_proc_address(cstring.as_c_str())
    }
    fn event_pump(&mut self) -> Result<impl EventPump, Self::Error> {
        Ok(X11EventPump {
            conn: Arc::clone(&self.conn),
            win: self.window,
            keycode_table: self.keycode_table
        })    
    }
}

pub struct X11EventPump {
    conn: Arc<Connection>,
    win: x::Window,
    keycode_table: [key::Code; 256],
}

impl EventPump for X11EventPump {
    type Error = X11Error;
    fn wait_for_event(&self) -> Result<Event, Self::Error> {
        Ok(match self.conn.wait_for_event()? {
           xcb::Event::X(x::Event::Expose(_)) => Event::Redraw, 
           xcb::Event::X(x::Event::KeyPress(ev)) => {
                let xcode = ev.detail() as usize;
                if xcode >= self.keycode_table.len() {
                    eprintln!("Unknown key {xcode}");
                    Event::KeyPress(key::Code::Unknown);
                }

                Event::KeyPress(self.keycode_table[xcode])
           },
           _ => Event::Unknown,
        })
    }
}

fn build_keycode_table() -> [key::Code; 256] {
    [
        // 0x00     0
        key::Code::Unknown,
        key::Code::Unknown,
        key::Code::Unknown,
        key::Code::Unknown,
        key::Code::Unknown,
        key::Code::Unknown,
        key::Code::Unknown,
        key::Code::Unknown,
        key::Code::Unknown,
        key::Code::Escape,
        key::Code::N1,
        key::Code::N2,
        key::Code::N3,
        key::Code::N4,
        key::Code::N5,
        key::Code::N6,
        // 0x10     16
        key::Code::N7,
        key::Code::N8,
        key::Code::N9,
        key::Code::N0,
        key::Code::Minus,
        key::Code::Equals,
        key::Code::Backspace,
        key::Code::Tab,
        key::Code::Q,
        key::Code::W,
        key::Code::E,
        key::Code::R,
        key::Code::T,
        key::Code::Y,
        key::Code::U,
        key::Code::I,
        // 0x20     32
        key::Code::O,
        key::Code::P,
        key::Code::LeftBracket,
        key::Code::RightBracket,
        key::Code::Enter,
        key::Code::LeftCtrl,
        key::Code::A,
        key::Code::S,
        key::Code::D,
        key::Code::F,
        key::Code::G,
        key::Code::H,
        key::Code::J,
        key::Code::K,
        key::Code::L,
        key::Code::Semicolon,
        // 0x30     48
        key::Code::Quote,
        key::Code::Grave,
        key::Code::LeftShift,
        key::Code::UK_Hash,
        key::Code::Z,
        key::Code::X,
        key::Code::C,
        key::Code::V,
        key::Code::B,
        key::Code::N,
        key::Code::M,
        key::Code::Comma,
        key::Code::Period,
        key::Code::Slash,
        key::Code::RightShift,
        key::Code::KP_Multiply,
        // 0x40     64
        key::Code::LeftAlt,
        key::Code::Space,
        key::Code::CapsLock,
        key::Code::F1,
        key::Code::F2,
        key::Code::F3,
        key::Code::F4,
        key::Code::F5,
        key::Code::F6,
        key::Code::F7,
        key::Code::F8,
        key::Code::F9,
        key::Code::F10,
        key::Code::KP_NumLock,
        key::Code::ScrollLock,
        key::Code::KP_7,
        // 0x50     80
        key::Code::KP_8,
        key::Code::KP_9,
        key::Code::KP_Subtract,
        key::Code::KP_4,
        key::Code::KP_5,
        key::Code::KP_6,
        key::Code::KP_Add,
        key::Code::KP_1,
        key::Code::KP_2,
        key::Code::KP_3,
        key::Code::KP_0,
        key::Code::KP_Period,
        key::Code::Unknown,
        key::Code::Unknown,
        key::Code::UK_Backslash,
        key::Code::F11,
        // 0x60     96
        key::Code::F12,
        key::Code::Unknown,
        key::Code::LANG3,   // Katakana
        key::Code::LANG4,   // Hiragana
        key::Code::Unknown, // Henkan
        key::Code::Unknown, // Hiragana_Katakana
        key::Code::Unknown, // Muhenkan
        key::Code::Unknown,
        key::Code::KP_Enter,
        key::Code::RightCtrl,
        key::Code::KP_Divide,
        key::Code::PrintScreen,
        key::Code::RightAlt,
        key::Code::Unknown, // line feed
        key::Code::Home,
        key::Code::Up,
        // 0x70     112
        key::Code::PageUp,
        key::Code::Left,
        key::Code::Right,
        key::Code::End,
        key::Code::Down,
        key::Code::PageDown,
        key::Code::Insert,
        key::Code::Delete,
        key::Code::Unknown,
        key::Code::Mute,
        key::Code::VolumeDown,
        key::Code::VolumeUp,
        key::Code::Unknown, // power off
        key::Code::KP_Equal,
        key::Code::KP_PlusMinus,
        key::Code::Pause,
        // 0x80     128
        key::Code::Unknown, // launch A
        key::Code::KP_Decimal,
        key::Code::LANG1, // hangul
        key::Code::LANG2, // hangul/hanja toggle
        key::Code::Unknown,
        key::Code::Unknown,
        key::Code::Unknown,
        key::Code::Menu,
        key::Code::Cancel,
        key::Code::Again,
        key::Code::Unknown, // SunProps
        key::Code::Undo,
        key::Code::Unknown, // SunFront
        key::Code::Copy,
        key::Code::Unknown, // Open
        key::Code::Paste,
        // 0x90     144
        key::Code::Find,
        key::Code::Cut,
        key::Code::Help,
        key::Code::Unknown, // XF86MenuKB
        key::Code::Unknown, // XF86Calculator
        key::Code::Unknown,
        key::Code::Unknown, //XF86Sleep
        key::Code::Unknown, //XF86Wakeup
        key::Code::Unknown, //XF86Explorer
        key::Code::Unknown, //XF86Send
        key::Code::Unknown,
        key::Code::Unknown, //Xfer
        key::Code::Unknown, //launch1
        key::Code::Unknown, //launch2
        key::Code::Unknown, //WWW
        key::Code::Unknown, //DOS
        // 0xA0     160
        key::Code::Unknown, // Screensaver
        key::Code::Unknown,
        key::Code::Unknown, // RotateWindows
        key::Code::Unknown, // Mail
        key::Code::Unknown, // Favorites
        key::Code::Unknown, // MyComputer
        key::Code::Unknown, // Back
        key::Code::Unknown, // Forward
        key::Code::Unknown,
        key::Code::Unknown, // Eject
        key::Code::Unknown, // Eject
        key::Code::Unknown, // AudioNext
        key::Code::Unknown, // AudioPlay
        key::Code::Unknown, // AudioPrev
        key::Code::Unknown, // AudioStop
        key::Code::Unknown, // AudioRecord
        // 0xB0     176
        key::Code::Unknown, // AudioRewind
        key::Code::Unknown, // Phone
        key::Code::Unknown,
        key::Code::Unknown, // Tools
        key::Code::Unknown, // HomePage
        key::Code::Unknown, // Reload
        key::Code::Unknown, // Close
        key::Code::Unknown,
        key::Code::Unknown,
        key::Code::Unknown, // ScrollUp
        key::Code::Unknown, // ScrollDown
        key::Code::Unknown, // parentleft
        key::Code::Unknown, // parentright
        key::Code::Unknown, // New
        key::Code::Unknown, // Redo
        key::Code::Unknown, // Tools
        // 0xC0     192
        key::Code::Unknown, // Launch5
        key::Code::Unknown, // Launch6
        key::Code::Unknown, // Launch7
        key::Code::Unknown, // Launch8
        key::Code::Unknown, // Launch9
        key::Code::Unknown,
        key::Code::Unknown, // AudioMicMute
        key::Code::Unknown, // TouchpadToggle
        key::Code::Unknown, // TouchpadPadOn
        key::Code::Unknown, // TouchpadOff
        key::Code::Unknown,
        key::Code::Unknown, // Mode_switch
        key::Code::Unknown, // Alt_L
        key::Code::Unknown, // Meta_L
        key::Code::Unknown, // Super_L
        key::Code::Unknown, // Hyper_L
        // 0xD0     208
        key::Code::Unknown, // AudioPlay
        key::Code::Unknown, // AudioPause
        key::Code::Unknown, // Launch3
        key::Code::Unknown, // Launch4
        key::Code::Unknown, // LaunchB
        key::Code::Unknown, // Suspend
        key::Code::Unknown, // Close
        key::Code::Unknown, // AudioPlay
        key::Code::Unknown, // AudioForward
        key::Code::Unknown,
        key::Code::Unknown, // Print
        key::Code::Unknown,
        key::Code::Unknown, // WebCam
        key::Code::Unknown,
        key::Code::Unknown,
        key::Code::Unknown, // Mail
        // 0xE0     224
        key::Code::Unknown, // Messenger
        key::Code::Unknown, // Seach
        key::Code::Unknown, // GO
        key::Code::Unknown, // Finance
        key::Code::Unknown, // Game
        key::Code::Unknown, // Shop
        key::Code::Unknown,
        key::Code::Unknown, // Cancel
        key::Code::Unknown, // MonBrightnessDown
        key::Code::Unknown, // MonBrightnessUp
        key::Code::Unknown, // AudioMedia
        key::Code::Unknown, // Display
        key::Code::Unknown, // KbdLightOnOff
        key::Code::Unknown, // KbdBrightnessDown
        key::Code::Unknown, // KbdBrightnessUp
        key::Code::Unknown, // Send
        // 0xF0     240
        key::Code::Unknown, // Reply
        key::Code::Unknown, // MailForward
        key::Code::Unknown, // Save
        key::Code::Unknown, // Documents
        key::Code::Unknown, // Battery
        key::Code::Unknown, // Bluetooth
        key::Code::Unknown, // WLan
        key::Code::Unknown,
        key::Code::Unknown,
        key::Code::Unknown,
        key::Code::Unknown,
        key::Code::Unknown,
        key::Code::Unknown,
        key::Code::Unknown,
        key::Code::Unknown,
        key::Code::Unknown,
    ]
}

