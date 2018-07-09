use std::io::{stdout, Write};
use std::os::unix::io::AsRawFd;
//use std::default::Default;
use ::libc::{STDOUT_FILENO};
use ::termios::{Termios, tcgetattr, tcsetattr, cfmakeraw};

// Because Rust won't escape "\033" in a string to 27
const ESCAPE: char = 27 as char;
const BEL: char = 7 as char;
const IEXTEN: u32 = 0100000;
const TCSANOW: i32 = 0;

// This doesn't seem to be found in the `libc` crate, so just re-define it here anyway.
const TIOCGWINSZ: ::libc::c_ulong = 0x00005413;

pub struct Screen {
	turn_on: String,
	turn_off: String,
	dims: TermDim,
	cur_pos: TermDim,
	term_original: Termios,
	term_settings: Termios,
	term_descript: i32,
	cursor_state: CursorState,
	state_mode: ModeState,
}

impl Screen {
	pub fn new() -> Option<Screen> {
		// TODO: Hard-code as little stuff as possible, return None if unable to get something that we require
		
		// Check if the output is a terminal, if not then it's impossible to build Screen
		if unsafe { ::libc::isatty(::libc::STDOUT_FILENO as i32) } == 0 {
			return None;
		}
		
		// Get as much info as possible and then build Screen
		let dims = TermDim::query().unwrap();
		
		let fd = match ::std::fs::File::open("/dev/tty") {
			Ok(e) => e,
			_ => return None
		};
		let term_state = match Termios::from_fd(fd.as_raw_fd()) { Ok(e) => e, _ => return None };

		let mut out = Screen {
			turn_on: format!("{0}7{0}[?1049h", ESCAPE),
			turn_off: format!("{0}[2J{0}[?1049l{0}8", ESCAPE),
			dims: dims,
			cur_pos: TermDim { height: 0, width: 0 },
			term_original: term_state,
			term_settings: term_state,
			term_descript: ::libc::STDOUT_FILENO,
			cursor_state: CursorState::Blinking, // Should always be defaulted to "Blinking"
			state_mode: ModeState::Default,
		};
		
		// For current reference http://cboard.cprogramming.com/linux-programming/158476-termios-examples.html
		
		// Grab a copy of the current Struct_termios
		if tcgetattr(out.term_descript, &mut out.term_original).is_err() || tcgetattr(out.term_descript, &mut out.term_settings).is_err() {
			println!("FAILURE!");
			return None;
		}
		
		// Turn the alt screen on
		print!("{}", out.turn_on);
		
		Some(out)
	}
	pub fn move_cursor(&mut self, y: u16, x: u16) {
		self.cur_pos.height = y;
		self.cur_pos.width = x;
		print!("{}[{};{}H", ESCAPE, y, x);
	}
	/// Sets the title of the terminal window.
	pub fn set_title(&self, title: &str) {
		print!("{}]2;{}{}", ESCAPE, title, BEL);
	}
	/// Sets the cursor's state.
	pub fn set_cursor(&mut self, flag: CursorState) {
		match flag {
			CursorState::Solid => {
				// TODO: Currently unimplemented
				return;
			},
			CursorState::Blinking => {
				if !self.cursor_state.is_blinking() {
					print!("{}[?25h", ESCAPE);
				}
			},
			CursorState::Off => {
				if !self.cursor_state.is_off() {
					print!("{}[?25l", ESCAPE);
				}
			}
		}
		self.cursor_state = flag;
	}
	/// Attempts to set the terminal's mode.
	/// If it fails, returns None
	/// 
	/// NOTE: Only setting it to raw mode is currently implemented.
	pub fn set_mode(&mut self, flag: ModeState) -> Option<()> {
		let out = match flag {
			ModeState::Default => {
				// TODO: Add code to disable what cfmakeraw does. (or do cfmakeraw manually)
				None
			},
			ModeState::Cbreak => {
				None
			},
			ModeState::Raw => {
				cfmakeraw(&mut self.term_settings);
				self.update_term()
			}
		};
		if out.is_some() {
			self.state_mode = flag;
		}
		out
	}
	/// Sets the terminal to how it was when creating this
	pub fn set_screen_default(&mut self) -> Option<()> {
		self.term_settings = self.term_original;
		self.update_term()
	}
	/// Temp: Just here in-case I need it.
	pub fn flush(&self) {
		stdout().flush().unwrap();
	}
	/// Internal: Attempts to set the termios struct
	fn update_term(&mut self) -> Option<()> {
		if tcsetattr(self.term_descript, TCSANOW, &self.term_settings).is_err() {
			None
		} else {
			Some(())
		}
	}
}

impl Drop for Screen {
	fn drop(&mut self) {
		self.set_cursor(CursorState::Blinking);
		self.set_screen_default().unwrap_or(());
		print!("{}", self.turn_off);
		self.flush();
	}
}

#[derive(Debug, Default, Clone)]
struct TermDim {
	height: u16,
	width: u16
}
impl TermDim {
	#[inline]
	/// Get the height (amount of lines) of the terminal
	pub fn get_height(&self) -> u16 {
		self.height
	}
	#[inline]
	/// Get the width (amount of columns) of the terminal
	pub fn get_width(&self) -> u16 {
		self.width
	}

	/// Queries the size of the terminal
	pub fn query() -> Option<TermDim> {
		let fd = match ::std::fs::File::open("/dev/tty") {
			Ok(e) => e,
			_ => return None
		};
		
		let ws = (0, 0);

		if unsafe { ::libc::ioctl(fd.as_raw_fd(), TIOCGWINSZ, &ws) } < 0 {
			// The query failed, return None
			return None;
		}

		Some(TermDim {
			height: ws.0,
			width: ws.1
		})
	}
}

/// The possible states for the Cursor
pub enum CursorState {
	/// Cursor is Solid
	Solid,
	/// Cursor is Blinking (default)
	Blinking,
	/// Cursor is disabled
	Off
}
impl CursorState {
	pub fn is_solid(&self) -> bool {
		match *self {
			CursorState::Solid => true,
			_ => false
		}
	}
	pub fn is_blinking(&self) -> bool {
		match *self {
			CursorState::Blinking => true,
			_ => false
		}
	}
	pub fn is_off(&self) -> bool {
		match *self {
			CursorState::Off => true,
			_ => false
		}
	}
}

/// Possible modes for the terminal to be in.
pub enum ModeState {
	/// The default mode for the terminal, typed text will go to the screen.
	Default,
	/// Typed text will get sent *only* to the screen, not to the screen.  Expected keystrokes to send signals (e.g., ctrl+c for SIGINT) will still work.
	Cbreak,
	/// Like cbreak, but even the signal keystrokes will get sent to the program w/o sending the signal.
	Raw,
}

