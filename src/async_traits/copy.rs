use core::future::Future;
use core::task::{Context, Poll};
use core::pin::Pin;
use core::marker::PhantomData;
use unchecked_ops::*;
use crate::AllError;

pub struct AsyncCopy<'a, 'b, R: ?Sized, W: ?Sized, E> {
    read: Pin<&'a mut R>,
    write: Pin<&'b mut W>,
    buffer: [u8; 0x10],
    buffer_read: usize,
    buffer_write: usize,
    total: usize,
    eof: bool,
    _err: PhantomData<fn() -> E>,
}

impl<'a, 'b, R: ?Sized, W: ?Sized, E> AsyncCopy<'a, 'b, R, W, E> {
    pub fn new(read: Pin<&'a mut R>, write: Pin<&'b mut W>) -> Self {
        Self {
            read,
            write,
            buffer: Default::default(),
            buffer_read: 0,
            buffer_write: 0,
            total: 0,
            eof: false,
            _err: PhantomData,
        }
    }
}

#[derive(Debug, Copy, Clone)]
#[cfg_attr(feature = "ufmt", derive(ufmt::derive::uDebug))]
enum State {
    Pending,
    Eof,
    Buffer,
    Ready,
}

impl<'a, 'b, R: ?Sized + super::AsyncRead, W: ?Sized + super::AsyncWrite, E: From<R::Error> + From<W::Error>> AsyncCopy<'a, 'b, R, W, E> {
    fn do_read(&mut self, cx: &mut Context) -> Result<State, E> {
        if self.eof {
            return Ok(State::Eof)
        }

        let buffer = self.buffer.get_mut(self.buffer_read..).unwrap_or(&mut []);
        if buffer.is_empty() {
            return Ok(State::Buffer)
        }

        match self.read.as_mut().poll_read(cx, buffer)? {
            Poll::Pending => Ok(State::Pending),
            Poll::Ready(0) => {
                self.eof = true;
                Ok(State::Eof)
            },
            Poll::Ready(len) => {
                unsafe {
                    debug_assert!(len <= buffer.len());
                    self.buffer_read = self.buffer_read.unchecked_add(len);
                }
                Ok(State::Ready)
            },
        }
    }

    fn do_write(&mut self, cx: &mut Context) -> Result<State, AllError<E>> {
        let buffer = self.buffer.get(self.buffer_write..self.buffer_read).unwrap_or(&[]);
        if buffer.is_empty() {
            return if self.eof {
                Ok(State::Eof)
            } else {
                Ok(State::Buffer)
            }
        }

        match self.write.as_mut().poll_write(cx, buffer).map_err(E::from)? {
            Poll::Pending => Ok(State::Pending),
            Poll::Ready(0) => Err(AllError::UnexpectedEof),
            Poll::Ready(len) => {
                unsafe {
                    debug_assert!(len <= buffer.len());
                    self.buffer_write = self.buffer_write.unchecked_add(len);
                }
                self.total = self.total.saturating_add(len);
                self.shuffle(); // TODO: only do this when buffer is full?
                Ok(State::Ready)
            },
        }
    }

    fn shuffle(&mut self) {
        self.buffer.copy_within(self.buffer_write..self.buffer_read, 0);
        unsafe {
            self.buffer_read = self.buffer_read.unchecked_sub(self.buffer_write);
        }
        self.buffer_write = 0;
    }
}

impl<'a, 'b, R: ?Sized + super::AsyncRead, W: ?Sized + super::AsyncWrite, E: From<R::Error> + From<W::Error>> Future for AsyncCopy<'a, 'b, R, W, E> {
    type Output = Result<usize, AllError<E>>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        let s = self.as_mut().get_mut();
        loop {
            let read = s.do_read(cx)?;
            let write = s.do_write(cx)?;

            match (read, write) {
                (_, State::Eof) =>
                    break Poll::Ready(Ok(self.total)),
                (_, State::Pending) | (State::Pending, State::Buffer) =>
                    break Poll::Pending,
                #[cfg(debug_assertions)]
                (State::Eof, State::Buffer) | (State::Buffer, State::Buffer) =>
                    panic!("invalid AsyncCopy state"),
                _ => (),
            }
        }
    }
}
