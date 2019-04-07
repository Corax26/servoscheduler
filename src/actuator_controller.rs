use std::fs::{File, OpenOptions};
use std::os::unix::prelude::FileExt;
use std::path::Path;
use std::sync::{Arc, Mutex};

use actuator::*;

pub trait ActuatorController {
    fn set_state(&mut self, state: &ActuatorState);
}
pub type ActuatorControllerHandle = Arc<Mutex<ActuatorController + Send>>;

pub struct FileActuatorController {
    file: File,
}

impl FileActuatorController {
    pub fn new(path: &Path) -> ::std::io::Result<ActuatorControllerHandle> {
        let file = OpenOptions::new().write(true).open(path)?;

        Ok(Arc::new(Mutex::new(FileActuatorController {
            file
        })))
    }
}

impl ActuatorController for FileActuatorController {
    fn set_state(&mut self, state: &ActuatorState) {
        let data = match state {
            ActuatorState::Toggle(value) => format!("{}", if *value { "1" } else { "0 " }),
            ActuatorState::FloatValue(value) => format!("{:.3}", value),
        }.into_bytes();

        match self.file.write_at(&data, 0) {
            Ok(size) if size != data.len() => {
                eprintln!("Short write: {} / {} B", size, data.len());
            },
            Err(e) => {
                eprintln!("Write failed: {}", e);
            },
            _ => (),
        };
    }
}
