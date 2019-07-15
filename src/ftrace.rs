use std::fs;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

pub struct Tracer {
    tracefs: &'static Path,

    last_err_kind: Option<io::ErrorKind>,
    last_err_path: Option<PathBuf>,
}

fn bool_to_str(b: bool) -> &'static str {
    match b {
        true => "1",
        false => "0",
    }
}

impl Tracer {
    pub fn new() -> Tracer {
        Tracer {
            tracefs: Path::new(""),
            last_err_kind: None,
            last_err_path: None,
        }
    }

    pub fn set_tracefs(&mut self, tracefs: &'static Path) {
        let trace = tracefs.join("trace");

        self.last_err_kind = self.path_test(&trace);
        if self.last_err_kind.is_some() {
            self.last_err_path = Some(trace);
        } else {
            self.tracefs = tracefs;
        }
    }

    pub fn has_err(&self) -> bool {
        self.last_err_kind.is_some()
    }

    pub fn get_err(&self) -> (&io::ErrorKind, &PathBuf) {
        (self.last_err_kind.as_ref().unwrap(), self.last_err_path.as_ref().unwrap())
    }

    fn path_err(&mut self, kind: io::ErrorKind, path: PathBuf) {
        self.last_err_kind = Some(kind);
        self.last_err_path = Some(path);
    }

    fn path_test(&self, path: &Path) -> Option<io::ErrorKind> {
        match fs::metadata(path) {
            Err(err) => Some(err.kind()),
            _ => None,
        }
    }

    fn path_truncate(&mut self, path: PathBuf) {
        if let Err(err) = fs::File::create(path.as_path()) {
            self.path_err(err.kind(), path);
        }
    }

    fn path_write(&mut self, path: PathBuf, val: &str) {
        match fs::File::create(path.as_path()) {
            Ok(mut file) => {
                if let Err(err) = file.write_all(val.as_bytes()) {
                    self.path_err(err.kind(), path);
                }
            }
            Err(err) => self.path_err(err.kind(), path),
        }
    }

    fn path_read(&mut self, path: PathBuf) -> String {
        let mut val = String::new();
        match fs::File::open(path.as_path()) {
            Ok(mut file) => {
                if let Err(err) = file.read_to_string(&mut val) {
                    self.path_err(err.kind(), path);
                }
            }
            Err(err) => self.path_err(err.kind(), path),
        }

        val
    }

    pub fn test(&self, path: &str) -> bool {
        let path = self.tracefs.join(path);
        self.path_test(&path).is_none()
    }

    pub fn truncate(&mut self, path: &str) {
        let path = self.tracefs.join(path);
        self.path_truncate(path);
    }

    pub fn write(&mut self, path: &str, val: &str) {
        let path = self.tracefs.join(path);
        self.path_write(path, val);
    }

    pub fn write_bool(&mut self, path: &str, val: bool) {
        self.write(path, bool_to_str(val));
    }

    pub fn write_i32(&mut self, path: &str, val: i32) {
        self.write(path, val.to_string().as_str());
    }

    pub fn read(&mut self, path: &str) -> String {
        let path = self.tracefs.join(path);
        self.path_read(path)
    }
}
