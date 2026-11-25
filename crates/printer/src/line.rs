//! The line to a printer: bytes out, and whatever comes back.
//!
//! Two lines reach a label printer. Its USB printer device, which the
//! kernel's `usblp` driver puts at `/dev/usb/lp0` and up, carries bytes
//! both ways. A TCP port, 9100 unless another is named, carries the
//! label; whether a printer also answers over it is the printer's
//! business, and nothing here counts on it.

use std::fs::{File, OpenOptions};
use std::io::{self, Read, Write};
use std::net::{Shutdown, TcpStream, ToSocketAddrs};
use std::path::{Path, PathBuf};
use std::thread;
use std::time::{Duration, Instant};

use rustix::event::{poll, PollFd, PollFlags, Timespec};

/// The port a label printer listens on for raw print data.
pub const RAW_PORT: u16 = 9100;

/// How long a TCP line may take to come up.
pub const CONNECT: Duration = Duration::from_secs(5);

/// How long a read of a device that has nothing to say is left alone
/// before it is tried again.
const IDLE: Duration = Duration::from_millis(20);

/// A line to a printer.
pub trait Line {
    /// Sends all of `bytes`.
    fn send(&mut self, bytes: &[u8]) -> io::Result<()>;

    /// Takes what has come back into `buffer`, waiting no longer than
    /// `wait` for the first of it. Returns how many bytes were taken:
    /// none when nothing came.
    fn receive(
        &mut self,
        buffer: &mut [u8],
        wait: Duration,
    ) -> io::Result<usize>;
}

/// Where a printer is reached.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Address {
    /// A USB printer device, by its path.
    Usb(PathBuf),
    /// A TCP port, as `host:port`.
    Tcp(String),
}

impl Address {
    /// The address of the TCP line to `host`: a name or an address, with
    /// or without a port. Without one it is [`RAW_PORT`].
    pub fn tcp(host: &str) -> Address {
        let bare_v6 = host.matches(':').count() > 1 && !host.starts_with('[');
        let has_port = !bare_v6
            && host
                .rsplit_once(':')
                .is_some_and(|(_, port)| port.parse::<u16>().is_ok());
        Address::Tcp(if has_port {
            host.to_string()
        } else if bare_v6 {
            format!("[{host}]:{RAW_PORT}")
        } else {
            format!("{host}:{RAW_PORT}")
        })
    }

    /// Whether a printer on this line is expected to answer. A USB
    /// printer device carries the printer's answers; a TCP port may not.
    pub fn answers(&self) -> bool {
        matches!(self, Address::Usb(_))
    }

    /// Opens the line.
    pub fn open(&self) -> io::Result<Box<dyn Line>> {
        Ok(match self {
            Address::Usb(path) => Box::new(UsbLine::open(path)?),
            Address::Tcp(host) => Box::new(TcpLine::open(host)?),
        })
    }
}

impl std::fmt::Display for Address {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Address::Usb(path) => write!(f, "USB device {}", path.display()),
            Address::Tcp(host) => write!(f, "TCP {host}"),
        }
    }
}

/// The line to a USB printer device.
#[derive(Debug)]
pub struct UsbLine {
    device: File,
}

impl UsbLine {
    /// Opens the printer device at `path` for both ways.
    pub fn open(path: &Path) -> io::Result<Self> {
        let device = OpenOptions::new().read(true).write(true).open(path)?;
        Ok(Self { device })
    }
}

impl Line for UsbLine {
    fn send(&mut self, bytes: &[u8]) -> io::Result<()> {
        self.device.write_all(bytes)?;
        self.device.flush()
    }

    fn receive(
        &mut self,
        buffer: &mut [u8],
        wait: Duration,
    ) -> io::Result<usize> {
        // The device reads as ready whenever the printer has completed a
        // transfer, an empty one included, so a read that brings nothing
        // is not the end of anything: it is tried again until the wait
        // is over.
        let until = Instant::now() + wait;
        loop {
            let left = until.saturating_duration_since(Instant::now());
            let timeout = Timespec {
                tv_sec: left.as_secs() as i64,
                tv_nsec: left.subsec_nanos().into(),
            };
            let mut waiting = [PollFd::new(&self.device, PollFlags::IN)];
            let ready = match poll(&mut waiting, Some(&timeout)) {
                Ok(ready) => ready,
                Err(rustix::io::Errno::INTR) => continue,
                Err(error) => return Err(error.into()),
            };
            if ready > 0 {
                match self.device.read(buffer) {
                    Ok(0) => {}
                    Ok(taken) => return Ok(taken),
                    Err(error)
                        if matches!(
                            error.kind(),
                            io::ErrorKind::WouldBlock
                                | io::ErrorKind::Interrupted
                        ) => {}
                    Err(error) => return Err(error),
                }
            }
            let left = until.saturating_duration_since(Instant::now());
            if left.is_zero() {
                return Ok(0);
            }
            thread::sleep(left.min(IDLE));
        }
    }
}

/// The line to a TCP port.
#[derive(Debug)]
pub struct TcpLine {
    stream: TcpStream,
}

impl TcpLine {
    /// Connects to `host`, a `host:port`, trying each address the name
    /// stands for and giving each no longer than [`CONNECT`].
    pub fn open(host: &str) -> io::Result<Self> {
        let mut last = io::Error::new(
            io::ErrorKind::NotFound,
            format!("{host} names no address"),
        );
        for address in host.to_socket_addrs()? {
            match TcpStream::connect_timeout(&address, CONNECT) {
                Ok(stream) => {
                    stream.set_nodelay(true)?;
                    return Ok(Self { stream });
                }
                Err(error) => last = error,
            }
        }
        Err(last)
    }
}

/// How long a line that is hung up goes on listening after the printer
/// last said something.
const QUIET: Duration = Duration::from_millis(300);

/// How long a line that is hung up listens at the most.
const HANGING_UP: Duration = Duration::from_secs(2);

impl Drop for TcpLine {
    /// Hangs up in order. The printer is told that nothing more comes,
    /// and what it still sends is taken until it hangs up too, or has
    /// been quiet for a moment.
    ///
    /// A line that is closed with something unread on it is not closed
    /// but reset by the system, and a reset may take with it what the
    /// printer has received and not yet read: the end of a label.
    fn drop(&mut self) {
        if self.stream.shutdown(Shutdown::Write).is_err() {
            return;
        }
        let until = Instant::now() + HANGING_UP;
        let mut unread = [0u8; 256];
        loop {
            let left = until.saturating_duration_since(Instant::now());
            if left.is_zero()
                || self.stream.set_read_timeout(Some(left.min(QUIET))).is_err()
            {
                return;
            }
            match self.stream.read(&mut unread) {
                // The printer hung up, or is quiet, or the line failed.
                Ok(0) | Err(_) => return,
                Ok(_) => {}
            }
        }
    }
}

impl Line for TcpLine {
    fn send(&mut self, bytes: &[u8]) -> io::Result<()> {
        self.stream.write_all(bytes)?;
        self.stream.flush()
    }

    fn receive(
        &mut self,
        buffer: &mut [u8],
        wait: Duration,
    ) -> io::Result<usize> {
        // A wait of nothing would mean "for ever" to the socket.
        let wait = wait.max(Duration::from_millis(1));
        self.stream.set_read_timeout(Some(wait))?;
        match self.stream.read(buffer) {
            Ok(taken) => Ok(taken),
            Err(error)
                if matches!(
                    error.kind(),
                    io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut
                ) =>
            {
                Ok(0)
            }
            Err(error) => Err(error),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpListener;

    #[test]
    fn a_tcp_address_gets_the_raw_port_unless_it_names_one() {
        for (given, expected) in [
            ("printer.example", "printer.example:9100"),
            ("192.0.2.7", "192.0.2.7:9100"),
            ("192.0.2.7:9101", "192.0.2.7:9101"),
            ("2001:db8::7", "[2001:db8::7]:9100"),
            ("[2001:db8::7]", "[2001:db8::7]:9100"),
            ("[2001:db8::7]:9101", "[2001:db8::7]:9101"),
        ] {
            assert_eq!(
                Address::tcp(given),
                Address::Tcp(expected.to_string()),
                "{given}"
            );
        }
    }

    #[test]
    fn only_a_usb_line_is_expected_to_answer() {
        assert!(Address::Usb("/dev/usb/lp0".into()).answers());
        assert!(!Address::tcp("192.0.2.7").answers());
        assert_eq!(
            Address::Usb("/dev/usb/lp0".into()).to_string(),
            "USB device /dev/usb/lp0"
        );
        assert_eq!(Address::tcp("192.0.2.7").to_string(), "TCP 192.0.2.7:9100");
    }

    /// A printer that has said something nobody read is not cut off in
    /// the middle of a label: the line is hung up, not reset, and the
    /// printer reads the label to its end.
    #[test]
    fn a_line_is_hung_up_in_order_with_something_unread_on_it() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap().to_string();
        let label = vec![0x5au8; 300_000];
        let whole = label.len();
        let printer = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            // It says something at once, which the sender never reads,
            // and reads slowly.
            stream.write_all(&[0xaa; 32]).unwrap();
            let mut heard = 0;
            let mut chunk = [0u8; 4096];
            loop {
                match stream.read(&mut chunk) {
                    Ok(0) => return Ok(heard),
                    Ok(taken) => heard += taken,
                    Err(error) => return Err((heard, error.kind())),
                }
                if heard < 50_000 {
                    thread::sleep(Duration::from_millis(1));
                }
            }
        });
        {
            let mut line = TcpLine::open(&address).unwrap();
            line.send(&label).unwrap();
        }
        assert_eq!(printer.join().unwrap(), Ok(whole));
    }

    #[test]
    fn a_tcp_line_carries_bytes_both_ways_and_waits_no_longer_than_told() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let far = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut asked = [0u8; 3];
            stream.read_exact(&mut asked).unwrap();
            stream.write_all(&[0x80, 0x20]).unwrap();
            let mut rest = Vec::new();
            stream.read_to_end(&mut rest).unwrap();
            (asked, rest)
        });

        let address = Address::tcp(&format!("127.0.0.1:{port}"));
        let mut line = address.open().unwrap();
        line.send(&[0x1b, 0x69, 0x53]).unwrap();
        let mut buffer = [0u8; 32];
        let taken = line.receive(&mut buffer, Duration::from_secs(5)).unwrap();
        assert_eq!(&buffer[..taken], &[0x80, 0x20]);

        let before = Instant::now();
        let taken = line
            .receive(&mut buffer, Duration::from_millis(50))
            .unwrap();
        assert_eq!(taken, 0, "nothing more was sent");
        assert!(before.elapsed() < Duration::from_secs(2));

        line.send(b"label").unwrap();
        drop(line);
        let (asked, rest) = far.join().unwrap();
        assert_eq!(asked, [0x1b, 0x69, 0x53]);
        assert_eq!(rest, b"label");
    }

    #[test]
    fn a_device_line_reads_what_is_there_and_gives_up_on_time() {
        // A named pipe stands in for the printer device: it can be opened
        // both ways by one process, and reads as not ready while empty.
        let path = std::env::temp_dir()
            .join(format!("evo-usher-line-{}", std::process::id()));
        let _ = std::fs::remove_file(&path);
        let made = std::process::Command::new("mkfifo").arg(&path).status();
        if !made.is_ok_and(|status| status.success()) {
            eprintln!("no mkfifo here; the device line was not exercised");
            return;
        }
        let mut line = UsbLine::open(&path).unwrap();
        let mut buffer = [0u8; 8];

        let before = Instant::now();
        let taken = line
            .receive(&mut buffer, Duration::from_millis(60))
            .unwrap();
        assert_eq!(taken, 0);
        assert!(before.elapsed() >= Duration::from_millis(60));
        assert!(before.elapsed() < Duration::from_secs(2));

        line.send(&[1, 2, 3]).unwrap();
        let taken = line.receive(&mut buffer, Duration::from_secs(5)).unwrap();
        assert_eq!(&buffer[..taken], &[1, 2, 3]);
        std::fs::remove_file(&path).unwrap();
    }
}
