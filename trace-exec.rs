extern crate collections;
extern crate libc;

use ptrace::Word;
use posix::CouldBeAnError; // needed for impl below

use std::os;
use std::str;
use std::mem;
use std::collections::hashmap::HashSet as HashSet;

mod posix;
mod ptrace;

enum TraceEvent {
    SystemCall(Word, Word, Word, Word, Word, Word, Word),
    Other,
}

enum TraceResult {
    TraceOk,
    TraceError(int),
}

impl CouldBeAnError for TraceResult {
    fn is_error(&self) -> bool {
        match *self {
            TraceError(_) => true,
            _             => false,
        }
    }

    fn get_error_as_string(&self) -> String {
        match *self {
            TraceError(errno) => posix::strerror(errno),
            _                 => "".to_string(),
        }
    }

    fn get_errno(&self) -> int {
        match *self {
            TraceError(errno) => errno,
            _                 => fail!("You can't get an errno from a success value!"),
        }
    }
}

fn wrap_result<T: CouldBeAnError>(result: T) -> TraceResult {
    if result.is_error() {
        TraceError(result.get_errno())
    } else {
        TraceOk
    }
}

fn init_trace(child_pid: int) -> TraceResult {
    match posix::waitpid(child_pid, 0) {
        posix::WaitPidFailure(errno)       => TraceError(errno),
        posix::WaitPidSuccess(pid, status) => {
            if status & posix::SIGTRAP != 0 {
                let result = ptrace::setoptions(pid, ptrace::TRACEFORK | ptrace::TRACESYSGOOD | ptrace::TRACEEXEC);
                if result.is_error() {
                    return wrap_result(result);
                }
                resume_trace(pid)
            } else {
                TraceError(0) // shit...
            }
        },
    }
}

fn resume_trace(child_pid: int) -> TraceResult {
    wrap_result(ptrace::syscall(child_pid))
}

struct TraceIterator {
    previous_pid: int
}

impl Iterator<(int, TraceEvent)> for TraceIterator {
    fn next(&mut self) -> Option<(int, TraceEvent)> {
        if self.previous_pid != -1 {
            resume_trace(self.previous_pid);
        }

        let result = posix::waitpid(-1, 0);

        match result {
            posix::WaitPidFailure(_)           => None,
            posix::WaitPidSuccess(pid, status) => {
                self.previous_pid = pid;

                if ((status >> 8) & (0x80 | posix::SIGTRAP)) != 0 {
                    match ptrace::get_registers(pid) {
                        Ok(ptrace::UserRegs { orig_rax: syscall_no, rdi: rdi, rsi: rsi, rdx: rdx, rcx: rcx, r8: r8, r9: r9, .. }) => {
                            Some((pid, SystemCall(syscall_no, rdi, rsi, rdx, rcx, r8, r9)))
                        },
                        Err(_) => None,
                    }
                } else {
                    Some((pid, Other))
                }
            },
        }
    }
}

fn next_trace() -> TraceIterator {
    TraceIterator {
        previous_pid: -1
    }
}

fn pstrdup(pid: int, addr: *const libc::c_void) -> String {
    let mut bytes    = vec![];
    let mut mut_addr = addr as Word;

    'outer: loop {
        match ptrace::peektext(pid, mut_addr as *const libc::c_void) {
            Err(_)   => break,
            Ok(word) => {
                let mut i = 0;

                // XXX I'm not using a for loop because of a bug in Rust
                while i < mem::size_of::<Word>() {
                    // XXX byte order
                    let lsb = (word >> (i * 8)) & 0xFF;
                    if lsb == 0 {
                        break 'outer;
                    }
                    bytes.push(lsb as u8);
                    i += 1;
                }
            }
        }
        mut_addr += mem::size_of::<Word>() as Word;
    }

    // XXX this is really a buffer of bytes rather than a string...
    match str::from_utf8(bytes.slice_from(0)) {
        None    => "", // XXX uh-oh...
        Some(s) => s,
    }.to_string()
}

fn get_program_args(pid: int, addr: *const libc::c_void) -> Vec<String> {
    let mut args     = vec![];
    let mut mut_addr = addr as Word;

    loop {
        match ptrace::peektext(pid, mut_addr as *const libc::c_void) {
            Err(_) | Ok(0) => break,
            Ok(word)       => {
                args.push(pstrdup(pid, word as *const libc::c_void));
            }
        }

        mut_addr += mem::size_of::<Word>() as Word;
    }

    args
}

fn handle_syscall_arguments(pid: int, (_, argv_ptr, _, _, _, _): (Word, Word, Word, Word, Word, Word)) {
    let argv = get_program_args(pid, argv_ptr as *const libc::c_void);
    println!("executable args: '{}'", argv);
}

fn run_parent(child_pid: int) -> TraceResult {
    let result = init_trace(child_pid);

    if result.is_error() {
        return wrap_result(result);
    }

    let mut awaiting_return        : HashSet<int> = HashSet::new();
    let mut seen_first_exec_return : HashSet<int> = HashSet::new();

    for (pid, event) in next_trace() {
        match event {
            SystemCall(ptrace::syscall::EXECVE, rdi, rsi, rdx, rcx, r8, r9) => {
                if awaiting_return.contains(&pid) {
                    if seen_first_exec_return.contains(&pid) {
                        awaiting_return.remove(&pid);
                        seen_first_exec_return.remove(&pid);
                    } else {
                        seen_first_exec_return.insert(pid);
                    }
                } else {
                    handle_syscall_arguments(pid, (rdi, rsi, rdx, rcx, r8, r9));
                    awaiting_return.insert(pid);
                }
            }
            _ => (),
        }
    }

    TraceOk
}

fn main() {
    let result = posix::fork();

    match result {
        posix::ForkChild => {
            let args   = os::args();
            let result = ptrace::trace_me();

            if result.is_error() {
                posix::exit(255);
            }
            posix::exec(args.tail());
            posix::exit(255);
        }
        posix::ForkFailure(_) => {
            println!("An error occurred: {}", result.get_error_as_string());
        }
        posix::ForkParent(child_pid) => {
            let result = run_parent(child_pid);

            if result.is_error() {
                posix::kill(child_pid, posix::SIGKILL);
                println!("An error occurred: {}", result.get_error_as_string());
            }
        }
    }
}
