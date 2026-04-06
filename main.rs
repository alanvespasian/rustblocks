mod blocks;
use blocks::{BLOCKS, DELIM};

use std::io::{self, Write};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use std::thread;

#[cfg(not(feature = "no_x"))]
mod x11 {
    use std::ffi::CString;
    use std::ptr;
    use x11::xlib;

    pub struct XState {
        pub dpy: *mut xlib::Display,
        pub root: xlib::Window,
    }

    unsafe impl Send for XState {}
    unsafe impl Sync for XState {}

    impl XState {
        pub fn setup() -> Self {
            unsafe {
                let dpy = xlib::XOpenDisplay(ptr::null());
                assert!(!dpy.is_null(), "dwmblocks: Failed to open display");
                let screen = xlib::XDefaultScreen(dpy);
                let root = xlib::XRootWindow(dpy, screen);
                XState { dpy, root }
            }
        }

        pub fn set_name(&self, name: &str) {
            unsafe {
                let cname = CString::new(name).unwrap();
                x11::xlib::XStoreName(self.dpy, self.root, cname.as_ptr());
                x11::xlib::XFlush(self.dpy);
            }
        }
    }

    impl Drop for XState {
        fn drop(&mut self) {
            unsafe { xlib::XCloseDisplay(self.dpy); }
        }
    }
}

const CMD_LEN: usize = 50;

static STATUS_CONTINUE: AtomicBool = AtomicBool::new(true);

fn getcmd(block: &blocks::Block, delim: &str) -> String {
    let output = Command::new("sh")
        .arg("-c")
        .arg(block.command)
        .stdout(Stdio::piped())
        .output()
        .unwrap();

    let text = String::from_utf8_lossy(&output.stdout)
        .trim_end_matches('\n')
        .to_string();

    if text.is_empty() && block.icon.is_empty() {
        return String::new();
    }

    let combined = format!("{}{}", block.icon, text);
    let truncated = &combined[..combined.len().min(CMD_LEN)];

    if !delim.is_empty() {
        format!("{}{}", truncated, delim)
    } else {
        truncated.to_string()
    }
}

fn getcmds(time: i64, statusbar: &mut Vec<String>, delim: &str) {
    for (i, block) in BLOCKS.iter().enumerate() {
        if time == -1 || (block.interval != 0 && (time as u32) % block.interval == 0) {
            statusbar[i] = getcmd(block, delim);
        }
    }
}

fn getsigcmds(signal: u32, statusbar: &mut Vec<String>, delim: &str) {
    for (i, block) in BLOCKS.iter().enumerate() {
        if block.signal == signal {
            statusbar[i] = getcmd(block, delim);
        }
    }
}

fn getstatus(statusbar: &[String], delim: &str) -> String {
    let full = statusbar.concat();
    if !delim.is_empty() && full.ends_with(delim) {
        full[..full.len() - delim.len()].to_string()
    } else {
        full
    }
}

fn pstdout(status: &str) {
    println!("{}", status);
    io::stdout().flush().ok();
}

#[cfg(not(feature = "no_x"))]
fn setroot(x: &x11::XState, status: &str) {
    x.set_name(status);
}

fn statusloop(
    delim: String,
    #[cfg(not(feature = "no_x"))] x: x11::XState,
    use_stdout: bool,
) {
    use signal_hook::iterator::Signals;
    use signal_hook::consts::signal::*;

    let mut sigs = vec![SIGTERM, SIGINT];
    for block in BLOCKS {
        if block.signal > 0 {
            sigs.push(libc::SIGRTMIN() + block.signal as i32);
        }
    }

    let mut statusbar = vec![String::new(); BLOCKS.len()];

    // Spawn signal handler thread with its own copy of statusbar state
    {
        let delim = delim.clone();
        thread::spawn(move || {
            let mut sb = vec![String::new(); BLOCKS.len()];
            let mut signals = Signals::new(&sigs).unwrap();
            for sig in signals.forever() {
                match sig {
                    s if s == SIGTERM || s == SIGINT => {
                        STATUS_CONTINUE.store(false, Ordering::SeqCst);
                        return;
                    }
                    s => {
                        let block_sig = (s - libc::SIGRTMIN()) as u32;
                        getsigcmds(block_sig, &mut sb, &delim);
                        let status = getstatus(&sb, &delim);
                        if use_stdout {
                            pstdout(&status);
                        } else {
                            #[cfg(not(feature = "no_x"))]
                            setroot(&x11::XState::setup(), &status);
                        }
                    }
                }
            }
        });
    }

    let mut i: u32 = 0;
    loop {
        getcmds(i as i64, &mut statusbar, &delim);
        let status = getstatus(&statusbar, &delim);
        if use_stdout {
            pstdout(&status);
        } else {
            #[cfg(not(feature = "no_x"))]
            setroot(&x, &status);
        }

        if !STATUS_CONTINUE.load(Ordering::SeqCst) {
            break;
        }

        thread::sleep(Duration::from_secs(1));
        i = i.wrapping_add(1);
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut delim = DELIM.to_string();
    let mut use_stdout = false;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "-d" => {
                i += 1;
                if i < args.len() {
                    delim = args[i].clone();
                }
            }
            "-p" => use_stdout = true,
            _ => {}
        }
        i += 1;
    }

    statusloop(
        delim,
        #[cfg(not(feature = "no_x"))]
        x11::XState::setup(),
        use_stdout,
    );
}
