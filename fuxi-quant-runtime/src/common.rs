use rhai::{EvalAltResult, Locked, Shared};
use std::{fmt::Display, ops::DerefMut};

pub type RTResult<T> = std::result::Result<T, Box<EvalAltResult>>;

#[inline(always)]
pub fn to_rt_err<T: Display>(t: T) -> Box<EvalAltResult> {
    t.to_string().into()
}

pub type SharedLocked<T> = Shared<Locked<T>>;

#[inline(always)]
pub fn new_shared_locked<T>(v: T) -> SharedLocked<T> {
    Shared::new(Locked::new(v))
}

#[inline(always)]
pub fn borrow_mut<T>(v: &Shared<Locked<T>>) -> impl DerefMut<Target = T> + '_ {
    v.borrow_mut()
}
