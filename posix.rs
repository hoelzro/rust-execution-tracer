use std::libc;
use std::os;
use std::ptr;
use std::str;
use std::vec;

mod c {
    use std::libc;

    extern {
        fn fork() -> libc::pid_t;
        fn exit(status: libc::c_int) -> !;
        fn getpid() -> libc::pid_t;
        fn waitpid(pid: libc::pid_t, status: *libc::c_int, flags: libc::c_int) -> libc::c_int;
        fn execvp(file: *libc::c_char, argv: **libc::c_char) -> !;
        fn kill(pid: libc::pid_t, signal: libc::c_int) -> libc::c_int;
        fn strerror(errno: libc::c_int) -> *libc::c_char;
    }
}

pub trait CouldBeAnError {
    fn is_error(&self) -> bool;
    fn get_error_as_string(&self) -> ~str;
    fn get_errno(&self) -> int;
}

pub enum PosixResult {
    PosixOk,
    PosixError(int),
}

#[fixed_stack_segment]
pub fn strerror(errno: int) -> ~str {
    unsafe {
        str::raw::from_c_str(c::strerror(errno as libc::c_int))
    }
}

impl CouldBeAnError for PosixResult {
    fn is_error(&self) -> bool {
        match *self {
            PosixOk       => false,
            PosixError(_) => true,
        }
    }

    fn get_error_as_string(&self) -> ~str {
        match *self {
            PosixOk           => ~"no error",
            PosixError(errno) => strerror(errno),
        }
    }

    fn get_errno(&self) -> int {
        match *self {
            PosixOk           => fail!(~"You can't get an errno from a success value!"),
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

    fn get_error_as_string(&self) -> ~str {
        match *self {
            ForkFailure(errno) => strerror(errno),
            _                  => ~"no error",
        }
    }

    fn get_errno(&self) -> int {
        match *self {
            ForkFailure(errno) => errno,
            _                  => fail!(~"You can't get an errno from a success value!"),
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

    fn get_error_as_string(&self) -> ~str {
        match *self {
            WaitPidFailure(errno) => strerror(errno),
            _                     => ~"no error",
        }
    }

    fn get_errno(&self) -> int {
        match *self {
            WaitPidFailure(errno) => errno,
            _                     => fail!(~"You can't get an errno from a success value!"),
        }
    }
}

#[fixed_stack_segment]
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

#[fixed_stack_segment]
pub fn getpid() -> int {
    unsafe {
        c::getpid() as int
    }
}

#[fixed_stack_segment]
pub fn waitpid(pid: int, flags: int) -> WaitPidResult {
    unsafe {
        let status : libc::c_int = 0;

        let pid = c::waitpid(pid as libc::pid_t, &status as *libc::c_int, flags as libc::c_int);

        if pid == -1 {
            WaitPidFailure(os::errno())
        } else {
            WaitPidSuccess(pid as int, status as int)
        }
    }
}

// this is probably pretty awful...
fn str_array_to_char_pp(ary: &[~str], callback: &fn(**libc::c_char)) {
    fn helper_fn(ptrs: &mut ~[*libc::c_char], ary: &[~str], callback: &fn(**libc::c_char)) {
        match ary {
            [] => {
                ptrs.push(ptr::null());
                callback(vec::raw::to_ptr(*ptrs));
            },
            [ref head, ..tail] => {
                do head.with_c_str |raw_str| {
                    ptrs.push(raw_str);
                }
                helper_fn(ptrs, tail, callback);
            },
        }
    }

    let mut ptrs : ~[*libc::c_char] = vec::with_capacity(ary.len());

    helper_fn(&mut ptrs, ary, callback);
}

#[fixed_stack_segment]
pub fn exec(command_and_args: &[~str]) {
    unsafe {
        do command_and_args[0].with_c_str |command| {
            do str_array_to_char_pp(command_and_args) |args| {
                c::execvp(command, args);
            }
        }
    }
}

#[fixed_stack_segment]
pub fn exit(status: int) -> ! {
    unsafe {
        c::exit(status as libc::c_int)
    }
}

#[fixed_stack_segment]
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
