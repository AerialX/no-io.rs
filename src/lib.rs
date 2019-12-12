#![cfg_attr(not(feature = "std"), no_std)]

use core::{fmt, cmp, convert};
#[cfg(feature = "std")]
use std::error::Error as StdError;

#[cfg(feature = "sync")]
mod sync_traits;
#[cfg(feature = "sync")]
pub use sync_traits::*;

#[cfg(feature = "async")]
mod async_traits;
#[cfg(feature = "async")]
pub use async_traits::*;

#[cfg(feature = "hash-stream")]
mod hash_stream;
#[cfg(feature = "hash-stream")]
pub use hash_stream::*;

pub mod prelude {
    #[cfg(feature = "sync")]
    pub use super::sync_traits::prelude::*;

    #[cfg(feature = "async")]
    pub use super::async_traits::prelude::*;
}

#[derive(Debug, Copy, Clone)]
#[cfg_attr(feature = "ufmt", derive(ufmt::derive::uDebug))]
pub struct Sink;

#[derive(Debug, Copy, Clone)]
#[cfg_attr(feature = "ufmt", derive(ufmt::derive::uDebug))]
pub struct Empty;

#[derive(Debug, Copy, Clone)]
#[cfg_attr(feature = "ufmt", derive(ufmt::derive::uDebug))]
pub struct Take<S> {
    stream: S,
    limit: usize,
}

impl<S> Take<S> {
    pub const fn new(stream: S, limit: usize) -> Self {
        Self {
            stream,
            limit,
        }
    }

    pub fn limit(&self) -> usize {
        self.limit
    }

    pub fn set_limit(&mut self, limit: usize) {
        self.limit = limit;
    }
}

#[derive(Debug, Copy, Clone, PartialOrd, Ord, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "ufmt", derive(ufmt::derive::uDebug))]
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
            AllError::UnexpectedEof => f.write_str("Unexpected EOF"),
            AllError::Io(e) => fmt::Display::fmt(e, f),
        }
    }
}

#[cfg(feature = "ufmt")]
impl<E: ufmt::uDisplay> ufmt::uDisplay for AllError<E> {
    fn fmt<W: ?Sized + ufmt::uWrite>(&self, f: &mut ufmt::Formatter<W>) -> Result<(), W::Error> {
        match self {
            AllError::UnexpectedEof => f.write_str("Unexpected EOF"),
            AllError::Io(e) => ufmt::uDisplay::fmt(e, f),
        }
    }
}

#[allow(non_camel_case_types)]
#[cfg(feature = "ufmt")]
pub struct uWriter<W: ?Sized> {
    inner: W,
}

#[cfg(feature = "ufmt")]
impl<W> uWriter<W> {
    pub fn new(inner: W) -> Self {
        Self {
            inner,
        }
    }

    pub fn into_inner(self) -> W {
        self.inner
    }
}

#[cfg(feature = "std")]
impl<E: StdError + 'static> StdError for AllError<E> {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self {
            AllError::Io(e) => Some(e),
            _ => None,
        }
    }
}

fn slice_read(this: &mut &[u8], buf: &mut [u8]) -> usize {
    let len = cmp::min(buf.len(), this.len());
    unsafe {
        buf.get_unchecked_mut(..len).copy_from_slice(this.get_unchecked(..len));
        *this = this.get_unchecked(len..);
    }
    len
}

fn slice_write(this: &mut &mut [u8], buf: &[u8]) -> Result<usize, AllError<convert::Infallible>> {
    let len = cmp::min(buf.len(), this.len());
    if len == 0 {
        match buf.is_empty() {
            true => Ok(0),
            false => Err(AllError::UnexpectedEof),
        }
    } else {
        let next = core::mem::replace(this, &mut []);
        unsafe {
            next.get_unchecked_mut(..len).copy_from_slice(buf.get_unchecked(..len));
            *this = next.get_unchecked_mut(len..);
        }
        Ok(len)
    }
}
