// Returns a result based on whether `cond` is true. This is designed to be used with the ? operator, returning Err(e)
// when `cond` is false, and Ok(()) otherwise.
pub fn verify<E>(cond: bool, e: E) -> Result<(), E> {
    cond.then(|| {}).ok_or(e)
}
