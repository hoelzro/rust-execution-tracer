extern crate libc;
use std::os;
use std::ptr;
use std::string;

mod c {
    extern crate libc;

    extern {
        pub fn fork() -> libc::pid_t;
        pub fn exit(status: libc::c_int) -> !;
        pub fn getpid() -> libc::pid_t;
        pub fn waitpid(pid: libc::pid_t, status: *mut libc::c_int, flags: libc::c_int) -> libc::c_int;
        pub fn execvp(file: *const libc::c_char, argv: *const *const libc::c_char) -> !;
        pub fn kill(pid: libc::pid_t, signal: libc::c_int) -> libc::c_int;
        pub fn strerror(errno: libc::c_int) -> *const libc::c_char;
        pub fn strdup(s: *const libc::c_char) -> *const libc::c_char;
    }
}

pub trait CouldBeAnError {
    fn is_error(&self) -> bool;
    fn get_error_as_string(&self) -> String;
    fn get_errno(&self) -> int;
}

pub enum PosixResult {
    PosixOk,
    PosixError(int),
}

pub fn strerror(errno: int) -> String {
    unsafe {
        string::raw::from_buf(c::strerror(errno as libc::c_int) as *const u8)
    }
}

impl CouldBeAnError for PosixResult {
    fn is_error(&self) -> bool {
        match *self {
            PosixOk       => false,
            PosixError(_) => true,
        }
    }

    fn get_error_as_string(&self) -> String {
        match *self {
            PosixOk           => "no error".to_string(),
            PosixError(errno) => strerror(errno),
        }
    }

    fn get_errno(&self) -> int {
        match *self {
            PosixOk           => fail!("You can't get an errno from a success value!"),
            PosixError(errno) => errno,
        }
    }
}

pub enum ForkResult {
    ForkFailure(int),
    ForkChild,
    ForkParent(int),
}

impl CouldBeAnError for ForkResult {
    fn is_error(&self) -> bool {
        match *self {
            ForkFailure(_) => true,
            _              => false,
        }
    }

    fn get_error_as_string(&self) -> String {
        match *self {
            ForkFailure(errno) => strerror(errno),
            _                  => "no error".to_string(),
        }
    }

    fn get_errno(&self) -> int {
        match *self {
            ForkFailure(errno) => errno,
            _                  => fail!("You can't get an errno from a success value!"),
        }
    }
}

pub enum WaitPidResult {
    WaitPidFailure(int),
    WaitPidSuccess(int, int),
}

impl CouldBeAnError for WaitPidResult {
    fn is_error(&self) -> bool {
        match *self {
            WaitPidFailure(_) => true,
            _                 => false,
        }
    }

    fn get_error_as_string(&self) -> String {
        match *self {
            WaitPidFailure(errno) => strerror(errno),
            _                     => "no error".to_string(),
        }
    }

    fn get_errno(&self) -> int {
        match *self {
            WaitPidFailure(errno) => errno,
            _                     => fail!("You can't get an errno from a success value!"),
        }
    }
}

pub fn fork() -> ForkResult {
    unsafe {
        let pid = c::fork();

        match pid {
            -1  => ForkFailure(os::errno()),
            0   => ForkChild,
            pid => ForkParent(pid as int),
        }
    }
}

pub fn getpid() -> int {
    unsafe {
        c::getpid() as int
    }
}

pub fn waitpid(pid: int, flags: int) -> WaitPidResult {
    unsafe {
        let mut status : libc::c_int = 0;

        let pid = c::waitpid(pid as libc::pid_t, &mut status as *mut libc::c_int, flags as libc::c_int);

        if pid == -1 {
            WaitPidFailure(os::errno())
        } else {
            WaitPidSuccess(pid as int, status as int)
        }
    }
}

// this is probably pretty awful...
fn str_array_to_char_pp(ary: &[String], callback: |*const *const libc::c_char| -> ()) {
    fn helper_fn(ptrs: &mut Vec<*const libc::c_char>, ary: &[String], callback: |*const *const libc::c_char| -> ()) {
        match ary {
            [] => {
                ptrs.push(ptr::null());
                callback(ptrs.as_ptr());
            },
            [ref head, ..tail] => {
                head.with_c_str(|raw_str| {
                    unsafe {
                        let copy = c::strdup(raw_str);
                        assert!(copy.is_not_null());
                        ptrs.push(copy);
                    }
                });
                helper_fn(ptrs, tail, callback);
            },
        }
    }

    let mut ptrs : Vec<*const libc::c_char> = Vec::with_capacity(ary.len());

    helper_fn(&mut ptrs, ary, callback);

    unsafe {
        for ptr in ptrs.iter() {
            libc::free(*ptr as *mut libc::c_void);
        }
    }
}

pub fn exec(command_and_args: &[String]) {
    unsafe {
        command_and_args[0].with_c_str(|command| {
            str_array_to_char_pp(command_and_args, |args| {
                c::execvp(command, args);
            });
        });
    }
}

pub fn exit(status: int) -> ! {
    unsafe {
        c::exit(status as libc::c_int)
    }
}

pub fn kill(pid: int, signum: int) -> PosixResult {
    unsafe {
        match c::kill(pid as libc::pid_t, signum as libc::c_int) {
            -1 => PosixError(os::errno()),
            _  => PosixOk,
        }
    }
}

pub static SIGTRAP : int = 5;
pub static SIGKILL : int = 9;
pub static ECHILD  : int = 10;

fn main() {}
