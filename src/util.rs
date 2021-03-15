use std::fmt::Display;
use std::process;

// Returns a result based on whether `cond` is true. This is designed to be used with the ? operator, returning Err(e)
// when `cond` is false, and Ok(()) otherwise.
pub fn verify<E>(cond: bool, e: E) -> Result<(), E> {
    cond.then(|| {}).ok_or(e)
}

pub fn log_info<D: Display>(value: D) {
    println!("info: {}", value);
}

pub fn log_fatal<D: Display>(value: D) -> ! {
    eprintln!("error: {}", value);
    process::exit(1)
}
