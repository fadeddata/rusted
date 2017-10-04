extern crate nix;
#[macro_use]
extern crate lazy_static;

use nix::unistd::{read, write};
use nix::libc::{STDIN_FILENO, cc_t, NCCS};
use std::process::exit;
use nix::sys::termios::*;
use nix::libc::iscntrl;
use std::io::{self, Write};

/*** macros ***/

macro_rules! ctrl_key {
    ($x:expr) => (($x as u8) & 0x1f);
}

/*** data ***/

struct EditorConfig {
    termios_flags: (InputFlags, OutputFlags, ControlFlags, LocalFlags, [cc_t; NCCS])
}

lazy_static! {
    static ref EDITOR_CONFIG: EditorConfig = {
        let ot = tcgetattr(STDIN_FILENO).expect("main:tcgetattr");
        EditorConfig { termios_flags: (ot.input_flags, ot.output_flags, ot.control_flags, ot.local_flags, ot.control_chars) }
    };
}

/*** terminal ***/

fn die(reason: String) {
    editor_refresh_screen();
    reset_mode();
    writeln!(io::stderr(), "{}", reason);
    exit(1);
}

fn enable_raw_mode() {
    // force a deref of the lazy static to store original termios
    EDITOR_CONFIG.termios_flags.1;

    let mut raw = tcgetattr(STDIN_FILENO).expect("main:tcgetattr");

    let l_flags = ECHO | ICANON | IEXTEN | ISIG;
    let i_flags = BRKINT | ICRNL | INPCK | ISTRIP | IXON;
    let o_flags = OPOST;
    let c_flags = CS8;

    raw.local_flags.remove(l_flags);
    raw.input_flags.remove(i_flags);
    raw.output_flags.remove(o_flags);
    raw.control_flags.remove(c_flags);
    raw.control_chars[SpecialCharacterIndices::VMIN as usize] = 0;
    raw.control_chars[SpecialCharacterIndices::VTIME as usize] = 1;

    match tcsetattr(STDIN_FILENO, SetArg::TCSAFLUSH, &raw) {
        Result::Err(err) => {
            die(format!("enable_raw_mode:tcsetattr {}", err));
        }

        _ => {}
    }
}

fn reset_mode() {
    let mut ot = tcgetattr(STDIN_FILENO).expect("reset_mode:tcgetattr");

    ot.input_flags = EDITOR_CONFIG.termios_flags.0;
    ot.output_flags = EDITOR_CONFIG.termios_flags.1;
    ot.control_flags = EDITOR_CONFIG.termios_flags.2;
    ot.local_flags = EDITOR_CONFIG.termios_flags.3;
    ot.control_chars = EDITOR_CONFIG.termios_flags.4;

    match tcsetattr(STDIN_FILENO, SetArg::TCSAFLUSH, &ot) {
        Result::Err(err) => {
            die(format!("reset_mode:tcsetattr {}", err));
        }

        _ => {}
    }
}


fn editor_read_key() -> u8 {
    let mut c: [u8; 1] = [0];

    loop {
        match read(STDIN_FILENO, &mut c) {
            Result::Ok(nread) if nread == 1 => {
                return c[0];
            }

            Result::Err(err) => {
                die(format!("editor_read_key:read {}", err));
            }

            _ => {}
        }
    }
}

/*** output ***/

fn write_escape(chars: Vec<char>) {
    let mut s = vec![0x1b];

    s.extend::<Vec<u8>>(chars.into_iter().map(|a| a as u8).collect());

    match write(STDIN_FILENO, &s) {
        Result::Err(err) => {
            die(format!("write_escape:write {}", err));
        }

        _ => {}
    }
}

fn write_chars(chars: Vec<char>) {
    let s: Vec<u8> = chars.into_iter().map(|a| a as u8).collect();

    match write(STDIN_FILENO, &s) {
        Result::Err(err) => {
            die(format!("write_chars:write {}", err));
        }

        _ => {}
    }
}

fn editor_draw_rows() {
    for _ in 0..24 {
        write_chars(vec!['~', '\r', '\n']);
    }
}

fn clear_screen() {
    write_escape(vec!['[', '2', 'J']);
    write_escape(vec!['[', 'H']);
}

fn editor_refresh_screen() {
    clear_screen();
    editor_draw_rows();
    write_escape(vec!['[', 'H']);
}

/*** input ***/

fn editor_process_keypress() {
    let c = editor_read_key();

    match c {
        _ if c == ctrl_key!('q') => {
            clear_screen();
            reset_mode();
            exit(0);            
        }

        _ => {}
    }
}

/*** init ***/

fn main() {
    enable_raw_mode();

    loop {
        editor_refresh_screen();
        editor_process_keypress();
    }
}
