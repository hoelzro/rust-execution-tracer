extern crate libc;
use std::ffi;
use std::os;
use std::ptr;
use std::str;

mod c {
    extern crate libc;

    extern {
        pub fn fork() -> libc::pid_t;
        pub fn exit(status: libc::c_int) -> !;
        pub fn waitpid(pid: libc::pid_t, status: *mut libc::c_int, flags: libc::c_int) -> libc::c_int;
        pub fn execvp(file: *const libc::c_char, argv: *const *const libc::c_char) -> !;
        pub fn kill(pid: libc::pid_t, signal: libc::c_int) -> libc::c_int;
        pub fn strerror(errno: libc::c_uint) -> *const libc::c_char;
        pub fn strdup(s: *const libc::c_char) -> *const libc::c_char;
    }
}

pub trait CouldBeAnError {
    fn is_error(&self) -> bool;
    fn get_error_as_string(&self) -> String;
    fn get_errno(&self) -> uint;
}

pub enum PosixResult {
    PosixOk,
    PosixError(uint),
}

pub fn strerror(errno: uint) -> String {
    unsafe {
        let c_error = c::strerror(errno as libc::c_uint);
        str::from_utf8_unchecked(ffi::c_str_to_bytes(&c_error)).to_string()
    }
}

impl CouldBeAnError for PosixResult {
    fn is_error(&self) -> bool {
        match *self {
            PosixResult::PosixOk       => false,
            PosixResult::PosixError(_) => true,
        }
    }

    fn get_error_as_string(&self) -> String {
        match *self {
            PosixResult::PosixOk           => "no error".to_string(),
            PosixResult::PosixError(errno) => strerror(errno),
        }
    }

    fn get_errno(&self) -> uint {
        match *self {
            PosixResult::PosixOk           => panic!("You can't get an errno from a success value!"),
            PosixResult::PosixError(errno) => errno,
        }
    }
}

pub enum ForkResult {
    ForkFailure(uint),
    ForkChild,
    ForkParent(int),
}

impl CouldBeAnError for ForkResult {
    fn is_error(&self) -> bool {
        match *self {
            ForkResult::ForkFailure(_) => true,
            _                          => false,
        }
    }

    fn get_error_as_string(&self) -> String {
        match *self {
            ForkResult::ForkFailure(errno) => strerror(errno),
            _                              => "no error".to_string(),
        }
    }

    fn get_errno(&self) -> uint {
        match *self {
            ForkResult::ForkFailure(errno) => errno,
            _                              => panic!("You can't get an errno from a success value!"),
        }
    }
}

pub enum WaitPidResult {
    WaitPidFailure(uint),
    WaitPidSuccess(int, int),
}

impl CouldBeAnError for WaitPidResult {
    fn is_error(&self) -> bool {
        match *self {
            WaitPidResult::WaitPidFailure(_) => true,
            _                                => false,
        }
    }

    fn get_error_as_string(&self) -> String {
        match *self {
            WaitPidResult::WaitPidFailure(errno) => strerror(errno),
            _                                    => "no error".to_string(),
        }
    }

    fn get_errno(&self) -> uint {
        match *self {
            WaitPidResult::WaitPidFailure(errno) => errno,
            _                                    => panic!("You can't get an errno from a success value!"),
        }
    }
}

pub fn fork() -> ForkResult {
    unsafe {
        let pid = c::fork();

        match pid {
            -1  => ForkResult::ForkFailure(os::errno()),
            0   => ForkResult::ForkChild,
            pid => ForkResult::ForkParent(pid as int),
        }
    }
}

pub fn waitpid(pid: int, flags: int) -> WaitPidResult {
    unsafe {
        let mut status : libc::c_int = 0;

        let pid = c::waitpid(pid as libc::pid_t, &mut status as *mut libc::c_int, flags as libc::c_int);

        if pid == -1 {
            WaitPidResult::WaitPidFailure(os::errno())
        } else {
            WaitPidResult::WaitPidSuccess(pid as int, status as int)
        }
    }
}

// this is probably pretty awful...
fn str_array_to_char_pp<Cb: Fn(*const *const libc::c_char) -> ()>(ary: &[String], callback: Cb) {
    fn helper_fn<Cb: Fn(*const *const libc::c_char) -> ()>(ptrs: &mut Vec<*const libc::c_char>, ary: &[String], callback: Cb) {
        match ary {
            [] => {
                ptrs.push(ptr::null());
                callback(ptrs.as_ptr());
            },
            [ref head, tail..] => {
                let raw_str = ffi::CString::from_slice(head.as_slice().as_bytes());
                unsafe {
                    let copy = c::strdup(raw_str.as_ptr());
                    assert!(!copy.is_null());
                    ptrs.push(copy);
                }
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
        let command = ffi::CString::from_slice(command_and_args[0].as_slice().as_bytes());
        str_array_to_char_pp(command_and_args, |&: args| {
            c::execvp(command.as_ptr(), args);
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
            -1 => PosixResult::PosixError(os::errno()),
            _  => PosixResult::PosixOk,
        }
    }
}

pub static SIGTRAP : int = 5;
pub static SIGKILL : int = 9;
