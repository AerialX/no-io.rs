#![cfg_attr(not(feature = "std"), no_std)]

use core::fmt;

#[cfg(feature = "sync")]
mod sync_traits;
#[cfg(feature = "sync")]
pub use sync_traits::*;

#[cfg(feature = "async")]
mod async_traits;
#[cfg(feature = "async")]
pub use async_traits::*;

#[derive(Debug, Copy, Clone, PartialOrd, Ord, PartialEq, Eq, Hash)]
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

impl<E: fmt::Display> fmt::Display for AllError<E> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            AllError::UnexpectedEof => write!(f, "Unexpected EOF"),
            AllError::Io(e) => fmt::Display::fmt(e, f),
        }
    }
}

#[cfg(feature = "std")]
use std::error::Error;

#[cfg(feature = "std")]
impl<E: Error + 'static> Error for AllError<E> {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            AllError::Io(e) => Some(e),
            _ => None,
        }
    }
}
