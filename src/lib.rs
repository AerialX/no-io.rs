#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "sync")]
mod sync_traits;
#[cfg(feature = "sync")]
pub use sync_traits::*;

#[cfg(feature = "async")]
mod async_traits;
#[cfg(feature = "async")]
pub use async_traits::*;

#[derive(Debug, Copy, Clone)]
pub enum AllError<E> {
    UnexpectedEof,
    Io(E),
}

impl<E> From<E> for AllError<E> {
    #[inline]
    fn from(e: E) -> Self {
        AllError::Io(e)
    }
}
