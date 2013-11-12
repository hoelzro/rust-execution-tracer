use posix::CouldBeAnError;
use std::cast;
use std::libc;
use std::os;
use std::ptr;

mod posix;

mod c {
    use std::libc;

    extern {
        fn ptrace(request: libc::c_int, pid: libc::pid_t, addr: *libc::c_void, data: *libc::c_void) -> libc::c_long;
    }
}

pub enum PtraceResult {
    PtraceOk,
    PtraceError(int),
}

impl CouldBeAnError for PtraceResult {
    fn is_error(&self) -> bool {
        match *self {
            PtraceError(_) => true,
            _              => false,
        }
    }

    fn get_error_as_string(&self) -> ~str {
        match *self {
            PtraceError(errno) => posix::strerror(errno),
            _                  => ~"no error",
        }
    }

    fn get_errno(&self) -> int {
        match *self {
            PtraceError(errno) => errno,
            _                  => fail!(~"You can't get an errno from a success value!"),
        }
    }
}

// XXX this type definition might not belong here
pub type word = u64;

// XXX extracting from headers would be nice
static TRACEME    : libc::c_int = 0;
static PEEKTEXT   : libc::c_int = 1;
static GETREGS    : libc::c_int = 12;
static SYSCALL    : libc::c_int = 24;
static SETOPTIONS : libc::c_int = 0x4200;

// XXX different on x86_64 vs x86
pub struct UserRegs {
  r15       : libc::uint64_t,
  r14       : libc::uint64_t,
  r13       : libc::uint64_t,
  r12       : libc::uint64_t,
  rbp       : libc::uint64_t,
  rbx       : libc::uint64_t,
  r11       : libc::uint64_t,
  r10       : libc::uint64_t,
  r9        : libc::uint64_t,
  r8        : libc::uint64_t,
  rax       : libc::uint64_t,
  rcx       : libc::uint64_t,
  rdx       : libc::uint64_t,
  rsi       : libc::uint64_t,
  rdi       : libc::uint64_t,
  orig_rax  : libc::uint64_t,
  rip       : libc::uint64_t,
  cs        : libc::uint64_t,
  eflags    : libc::uint64_t,
  rsp       : libc::uint64_t,
  ss        : libc::uint64_t,
  fs_base   : libc::uint64_t,
  gs_base   : libc::uint64_t,
  ds        : libc::uint64_t,
  es        : libc::uint64_t,
  fs        : libc::uint64_t,
  gs        : libc::uint64_t,
}

fn to_ptrace_result(return_value: libc::c_long) -> PtraceResult {
    match return_value {
        -1 => PtraceError(os::errno()),
        _  => PtraceOk,
    }
}

#[fixed_stack_segment]
pub fn trace_me() -> PtraceResult {
    unsafe {
        to_ptrace_result(c::ptrace(TRACEME, 0, ptr::null(), ptr::null()))
    }
}

#[fixed_stack_segment]
pub fn setoptions(pid: int, options: int) -> PtraceResult {
    unsafe {
        to_ptrace_result(c::ptrace(SETOPTIONS, pid as libc::pid_t, ptr::null(), options as *libc::c_void))
    }
}

#[fixed_stack_segment]
pub fn syscall(pid: int) -> PtraceResult {
    unsafe {
        to_ptrace_result(c::ptrace(SYSCALL, pid as libc::pid_t, ptr::null(), ptr::null()))
    }
}

// XXX this should probably return Result<~UserRegs, int>
#[fixed_stack_segment]
pub fn get_registers(pid: int) -> Result<UserRegs, int> {
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

        let result = c::ptrace(GETREGS, pid as libc::pid_t, ptr::null(), cast::transmute(&registers));

        if result == -1 {
            Err(os::errno())
        } else {
            Ok(registers)
        }
    }
}

#[fixed_stack_segment]
pub fn peektext(pid: int, addr: *libc::c_void) -> Result<word, int> {
    unsafe {
        let result = c::ptrace(PEEKTEXT, pid as libc::pid_t, addr, ptr::null());

        if result == -1 {
            let errno = os::errno();

            if errno != 0 {
                Err(errno)
            } else {
                Ok(result as word)
            }
        } else {
            Ok(result as word)
        }
    }
}

pub static TRACESYSGOOD : int = 0x00000001;
pub static TRACEFORK    : int = 0x00000002;
pub static TRACEEXEC    : int = 0x00000010;

pub mod syscall {
    pub static EXECVE : ::ptrace::word = 59;
}

fn main() {}
