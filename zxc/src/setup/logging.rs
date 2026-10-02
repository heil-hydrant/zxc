use std::fs::{File, create_dir};
use std::io::{self, Write};
use std::sync::{LockResult, Mutex, MutexGuard};

use time::UtcOffset;
use time::macros::format_description;
use tracing::subscriber::set_global_default;
use tracing::{Level, Metadata};
use tracing_subscriber::field::MakeExt;
use tracing_subscriber::fmt::time::OffsetTime;
use tracing_subscriber::fmt::{MakeWriter, format};

pub struct FileWriter<'a>(LockResult<MutexGuard<'a, File>>);

impl Write for FileWriter<'_> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.as_mut().unwrap().write(buf)
    }

    fn write_all(&mut self, buf: &[u8]) -> io::Result<()> {
        self.0.as_mut().unwrap().write_all(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.0.as_mut().unwrap().flush()
    }
}

// Custom Logger to write error to stderr and other traces to file
pub struct DebugLogger {
    basic: Mutex<File>,
    debug: Mutex<File>,
}

impl DebugLogger {
    #[inline(always)]
    pub fn new(basic: File, debug: File) -> Self {
        Self {
            basic: Mutex::new(basic),
            debug: Mutex::new(debug),
        }
    }
}

impl<'a> MakeWriter<'a> for DebugLogger {
    type Writer = FileWriter<'a>;

    fn make_writer(&'a self) -> Self::Writer {
        FileWriter(self.debug.lock())
    }

    fn make_writer_for(&'a self, meta: &Metadata<'_>) -> Self::Writer {
        if meta.level() <= &Level::DEBUG {
            return FileWriter(self.basic.lock());
        }

        FileWriter(self.debug.lock())
    }
}

pub struct BasicLogger(Mutex<File>);

impl<'a> MakeWriter<'a> for BasicLogger {
    type Writer = FileWriter<'a>;

    fn make_writer(&'a self) -> Self::Writer {
        FileWriter(self.0.lock())
    }

    fn make_writer_for(&'a self, _: &Metadata<'_>) -> Self::Writer {
        FileWriter(self.0.lock())
    }
}

// Setup logging
pub fn setup_logging(debug: bool) -> Result<(), Box<dyn std::error::Error>> {
    let time_format = format_description!("[hour]:[minute]:[second]");
    let time_offset =
        UtcOffset::current_local_offset().unwrap_or(UtcOffset::UTC);
    let timer = OffsetTime::new(time_offset, time_format);

    let formatter =
        format::debug_fn(|writer, _, value| write!(writer, "{:?}", value))
            .delimited(" | ");
    // Subscriber
    let mut subscriber = tracing_subscriber::fmt()
        .fmt_fields(formatter)
        .with_timer(timer);

    let max_level = if debug {
        Level::TRACE
    } else {
        Level::DEBUG
    };

    let basic_file = File::create("log.txt")?;
    subscriber = subscriber
        .with_max_level(max_level)
        .with_ansi(false);

    if max_level == Level::TRACE {
        let _ =
            create_dir("log").map_err(|e| eprintln!("create log dir| {}", e));
        match File::create("./log/proxy.log") {
            Ok(file) => {
                let writer = DebugLogger::new(basic_file, file);
                let subscriber = subscriber
                    .with_writer(writer)
                    .with_target(false)
                    .finish();
                set_global_default(subscriber)
                    .expect("setting default subscriber failed");
                return Ok(());
            }
            Err(e) => eprintln!("create debug log file| {}", e),
        }
    }
    let writer = BasicLogger(Mutex::new(basic_file));
    let subscriber = subscriber
        .with_target(false)
        .with_writer(writer)
        .finish();
    set_global_default(subscriber)?;
    Ok(())
}
