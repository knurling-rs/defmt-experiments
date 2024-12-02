//! [`defmt`](https://github.com/knurling-rs/defmt) global logger to a file on disk.

use std::{io::{self, Write}, fs::File, ops::DerefMut, path::Path, sync::{Condvar, Mutex}};

#[defmt::global_logger]
pub struct Logger;

struct State {
    file: Option<File>,
    encoder: defmt::Encoder,
    in_progress: bool,
}

impl State {
    const fn new() -> State {
        State {
            file: None,
            encoder: defmt::Encoder::new(),
            in_progress: false
        }
    }
}

/// Global logger lock.
static STATE: (Mutex<State>, Condvar) = (Mutex::new(State::new()), Condvar::new());

impl Logger {
    pub fn init<P: AsRef<Path>>(filename: P) -> io::Result<()> {
        let mut state = STATE.0.lock().unwrap();
        while state.in_progress {
            // sit on the condvar because only one thread can grab the lock
            state = STATE.1.wait(state).unwrap();
        }
        state.file = Some(File::create(filename)?);
        Ok(())
    }
}

unsafe impl defmt::Logger for Logger {
    fn acquire() {
        let mut state = STATE.0.lock().unwrap();
        while state.in_progress {
            // sit on the condvar because only one thread can grab the lock
            state = STATE.1.wait(state).unwrap();
        }
        let state = state.deref_mut();
        state.in_progress = true;
        state.encoder.start_frame(|b| {
            if let Some(f) = state.file.as_mut() {
                f.write_all(b).unwrap();
            }
        })
    }

    unsafe fn flush() {
        let mut state = STATE.0.lock().unwrap();
        if let Some(f) = state.file.as_mut() {
            f.flush().unwrap();
        }
    }

    unsafe fn release() {
        let mut state = STATE.0.lock().unwrap();
        let state = state.deref_mut();
        state.encoder.end_frame(|b| {
            if let Some(f) = state.file.as_mut() {
                f.write_all(b).unwrap();
            }
        });
        state.in_progress = false;
    }

    unsafe fn write(bytes: &[u8]) {
        let mut state = STATE.0.lock().unwrap();
        let state = state.deref_mut();
        state.encoder.write(bytes, |b| {
            if let Some(f) = state.file.as_mut() {
                f.write_all(b).unwrap();
            }
        });
    }
}

#[export_name = "_defmt_timestamp"]
fn defmt_timestamp(f: defmt::Formatter<'_>) {
    let now = std::time::SystemTime::now();
    if let Ok(delta) = now.duration_since(std::time::UNIX_EPOCH) {
        defmt::write!(f, "{=u64}", delta.as_millis() as u64);
    }
}
