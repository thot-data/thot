//! Funciontality for locking tests to prevent running in parallel.
use std::sync::{Mutex, MutexGuard};

#[macro_export]
macro_rules! create_lock {
    ( $id:ident ) => {
        lazy_static::lazy_static! {
            static ref $id: std::sync::Mutex<()> = std::sync::Mutex::new(());
        }
    };
}

pub fn get_lock(m: &'static Mutex<()>) -> MutexGuard<'static, ()> {
    match m.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}
