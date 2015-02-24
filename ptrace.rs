extern crate libc;

use posix::CouldBeAnError;
use std::mem;
use std::os;
use std::ptr;

use posix;

mod c {
    extern crate libc;

    extern {
        pub fn ptrace(request: libc::c_int, pid: libc::pid_t, addr: *const libc::c_void, data: *const libc::c_void) -> libc::c_long;
    }
}

pub enum PtraceResult {
    PtraceOk,
    PtraceError(uint),
}

impl CouldBeAnError for PtraceResult {
    fn is_error(&self) -> bool {
        match *self {
            PtraceResult::PtraceError(_) => true,
            _                            => false,
        }
    }

    fn get_error_as_string(&self) -> String {
        match *self {
            PtraceResult::PtraceError(errno) => posix::strerror(errno),
            _                                => "no error".to_string(),
        }
    }

    fn get_errno(&self) -> uint {
        match *self {
            PtraceResult::PtraceError(errno) => errno,
            _                                => panic!("You can't get an errno from a success value!"),
        }
    }
}

// XXX this type definition might not belong here
pub type Word = u64;

// XXX extracting from headers would be nice
static TRACEME    : libc::c_int = 0;
static PEEKTEXT   : libc::c_int = 1;
static GETREGS    : libc::c_int = 12;
static SYSCALL    : libc::c_int = 24;
static SETOPTIONS : libc::c_int = 0x4200;

// XXX different on x86_64 vs x86
pub struct UserRegs {
  pub r15       : libc::uint64_t,
  pub r14       : libc::uint64_t,
  pub r13       : libc::uint64_t,
  pub r12       : libc::uint64_t,
  pub rbp       : libc::uint64_t,
  pub rbx       : libc::uint64_t,
  pub r11       : libc::uint64_t,
  pub r10       : libc::uint64_t,
  pub r9        : libc::uint64_t,
  pub r8        : libc::uint64_t,
  pub rax       : libc::uint64_t,
  pub rcx       : libc::uint64_t,
  pub rdx       : libc::uint64_t,
  pub rsi       : libc::uint64_t,
  pub rdi       : libc::uint64_t,
  pub orig_rax  : libc::uint64_t,
  pub rip       : libc::uint64_t,
  pub cs        : libc::uint64_t,
  pub eflags    : libc::uint64_t,
  pub rsp       : libc::uint64_t,
  pub ss        : libc::uint64_t,
  pub fs_base   : libc::uint64_t,
  pub gs_base   : libc::uint64_t,
  pub ds        : libc::uint64_t,
  pub es        : libc::uint64_t,
  pub fs        : libc::uint64_t,
  pub gs        : libc::uint64_t,
}

fn to_ptrace_result(return_value: libc::c_long) -> PtraceResult {
    match return_value {
        -1 => PtraceResult::PtraceError(os::errno() as uint),
        _  => PtraceResult::PtraceOk,
    }
}

pub fn trace_me() -> PtraceResult {
    unsafe {
        to_ptrace_result(c::ptrace(TRACEME, 0, ptr::null(), ptr::null()))
    }
}

pub fn setoptions(pid: int, options: int) -> PtraceResult {
    unsafe {
        to_ptrace_result(c::ptrace(SETOPTIONS, pid as libc::pid_t, ptr::null(), options as *const libc::c_void))
    }
}

pub fn syscall(pid: int) -> PtraceResult {
    unsafe {
        to_ptrace_result(c::ptrace(SYSCALL, pid as libc::pid_t, ptr::null(), ptr::null()))
    }
}

// XXX this should probably return Result<~UserRegs, uint>
pub fn get_registers(pid: int) -> Result<UserRegs, uint> {
    unsafe {
        // XXX is there a better way to do this?
        let registers = UserRegs {
          r15       : 0,
          r14       : 0,
          r13       : 0,
          r12       : 0,
          rbp       : 0,
          rbx       : 0,
          r11       : 0,
          r10       : 0,
          r9        : 0,
          r8        : 0,
          rax       : 0,
          rcx       : 0,
          rdx       : 0,
          rsi       : 0,
          rdi       : 0,
          orig_rax  : 0,
          rip       : 0,
          cs        : 0,
          eflags    : 0,
          rsp       : 0,
          ss        : 0,
          fs_base   : 0,
          gs_base   : 0,
          ds        : 0,
          es        : 0,
          fs        : 0,
          gs        : 0,
        };

        let result = c::ptrace(GETREGS, pid as libc::pid_t, ptr::null(), mem::transmute(&registers));

        if result == -1 {
            Err(os::errno() as uint)
        } else {
            Ok(registers)
        }
    }
}

pub fn peektext(pid: int, addr: *const libc::c_void) -> Result<Word, uint> {
    unsafe {
        let result = c::ptrace(PEEKTEXT, pid as libc::pid_t, addr, ptr::null());

        if result == -1 {
            let errno = os::errno() as uint;

            if errno != 0 {
                Err(errno)
            } else {
                Ok(result as Word)
            }
        } else {
            Ok(result as Word)
        }
    }
}

pub const TRACESYSGOOD : int = 0x00000001;
pub const TRACEFORK    : int = 0x00000002;
pub const TRACEEXEC    : int = 0x00000010;

pub mod syscall {
    pub const EXECVE : super::Word = 59;
}
