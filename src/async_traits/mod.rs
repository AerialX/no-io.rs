use core::task::{Context, Poll};
use core::pin::Pin;

pub trait AsyncRead {
    type Error;

    fn poll_read(self: Pin<&mut Self>, cx: &mut Context, buf: &mut [u8]) -> Poll<Result<usize, Self::Error>>;
}

pub trait AsyncWrite {
    type Error;

    fn poll_write(self: Pin<&mut Self>, cx: &mut Context, buf: &[u8]) -> Poll<Result<usize, Self::Error>>;

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Result<(), Self::Error>>;
    fn poll_close(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Result<(), Self::Error>>;
}

pub trait AsyncSynchronous {
    type Error;

    fn poll_read_write(self: Pin<&mut Self>, context: &mut Context, buffer: &mut [u8]) -> Poll<Result<usize, Self::Error>>;
}

// TODO consider compat structs with pinned reference for borrowing rather than owning?

#[cfg(feature = "tokio-io")]
mod tokio_impl {
    use core::task::{Context, Poll};
    use core::pin::Pin;
    use std::io::Error;
    use tokio_io::{AsyncRead, AsyncWrite};

    pub struct TokioCompat<T: ?Sized>(pub T);

    impl<T: ?Sized> TokioCompat<T> {
        pub fn inner_pin(self: Pin<&mut Self>) -> Pin<&mut T> {
            unsafe {
                let this = self.get_unchecked_mut();
                Pin::new_unchecked(&mut this.0)
            }
        }
    }

    impl<T: ?Sized + AsyncRead> super::AsyncRead for TokioCompat<T> {
        type Error = Error;

        fn poll_read(self: Pin<&mut Self>, cx: &mut Context, buf: &mut [u8]) -> Poll<Result<usize, Self::Error>> {
            self.inner_pin().poll_read(cx, buf)
        }
    }

    impl<T: ?Sized + super::AsyncRead<Error=E>, E: Into<Error>> AsyncRead for TokioCompat<T> {
        fn poll_read(self: Pin<&mut Self>, cx: &mut Context, buf: &mut [u8]) -> Poll<Result<usize, Error>> {
            self.inner_pin().poll_read(cx, buf).map_err(Into::into)
        }
    }

    impl<T: ?Sized + AsyncWrite> super::AsyncWrite for TokioCompat<T> {
        type Error = Error;

        fn poll_write(self: Pin<&mut Self>, cx: &mut Context, buf: &[u8]) -> Poll<Result<usize, Self::Error>> {
            self.inner_pin().poll_write(cx, buf)
        }

        fn poll_flush(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Result<(), Self::Error>> {
            self.inner_pin().poll_flush(cx)
        }

        fn poll_close(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Result<(), Self::Error>> {
            self.inner_pin().poll_shutdown(cx)
        }
    }

    impl<T: ?Sized + super::AsyncWrite<Error=E>, E: Into<Error>> AsyncWrite for TokioCompat<T> {
        fn poll_write(self: Pin<&mut Self>, cx: &mut Context, buf: &[u8]) -> Poll<Result<usize, Error>> {
            self.inner_pin().poll_write(cx, buf).map_err(Into::into)
        }

        fn poll_flush(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Result<(), Error>> {
            self.inner_pin().poll_flush(cx).map_err(Into::into)
        }

        fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Result<(), Error>> {
            self.inner_pin().poll_close(cx).map_err(Into::into)
        }
    }
}

#[cfg(feature = "tokio-io")]
pub use tokio_impl::TokioCompat;

#[cfg(feature = "futures-io")]
mod futures_impl {
    use core::task::{Context, Poll};
    use core::pin::Pin;
    use std::io::Error;
    use futures_io::{AsyncRead, AsyncWrite};

    pub struct FuturesCompat<T: ?Sized>(pub T);

    impl<T: ?Sized> FuturesCompat<T> {
        pub fn inner_pin(self: Pin<&mut Self>) -> Pin<&mut T> {
            unsafe {
                let this = self.get_unchecked_mut();
                Pin::new_unchecked(&mut this.0)
            }
        }
    }

    impl<T: ?Sized + AsyncRead> super::AsyncRead for FuturesCompat<T> {
        type Error = Error;

        fn poll_read(self: Pin<&mut Self>, cx: &mut Context, buf: &mut [u8]) -> Poll<Result<usize, Self::Error>> {
            self.inner_pin().poll_read(cx, buf)
        }
    }

    impl<T: ?Sized + super::AsyncRead<Error=E>, E: Into<Error>> AsyncRead for FuturesCompat<T> {
        fn poll_read(self: Pin<&mut Self>, cx: &mut Context, buf: &mut [u8]) -> Poll<Result<usize, Error>> {
            self.inner_pin().poll_read(cx, buf).map_err(Into::into)
        }
    }

    impl<T: ?Sized + AsyncWrite> super::AsyncWrite for FuturesCompat<T> {
        type Error = Error;

        fn poll_write(self: Pin<&mut Self>, cx: &mut Context, buf: &[u8]) -> Poll<Result<usize, Self::Error>> {
            self.inner_pin().poll_write(cx, buf)
        }

        fn poll_flush(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Result<(), Self::Error>> {
            self.inner_pin().poll_flush(cx)
        }

        fn poll_close(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Result<(), Self::Error>> {
            self.inner_pin().poll_close(cx)
        }
    }

    impl<T: ?Sized + super::AsyncWrite<Error=E>, E: Into<Error>> AsyncWrite for FuturesCompat<T> {
        fn poll_write(self: Pin<&mut Self>, cx: &mut Context, buf: &[u8]) -> Poll<Result<usize, Error>> {
            self.inner_pin().poll_write(cx, buf).map_err(Into::into)
        }

        fn poll_flush(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Result<(), Error>> {
            self.inner_pin().poll_flush(cx).map_err(Into::into)
        }

        fn poll_close(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Result<(), Error>> {
            self.inner_pin().poll_close(cx).map_err(Into::into)
        }
    }
}

#[cfg(feature = "futures-io")]
pub use futures_impl::FuturesCompat;

// TODO these "all" poll impls can be shared to reduce code size

mod read_exact;
pub use read_exact::*;

mod write_all;
pub use write_all::*;

mod read_write_all;
pub use read_write_all::*;

trait BufferSlice {
    fn len(&self) -> usize;
    unsafe fn resize_from(&mut self, count: usize);
}

impl BufferSlice for &'_ mut [u8] {
    fn len(&self) -> usize {
        <[u8]>::len(self)
    }

    unsafe fn resize_from(&mut self, count: usize) {
        let buffer = core::mem::replace(self, &mut []);
        *self = buffer.get_unchecked_mut(count..);
    }
}

impl BufferSlice for &'_ [u8] {
    fn len(&self) -> usize {
        <[u8]>::len(self)
    }

    unsafe fn resize_from(&mut self, count: usize) {
        *self = self.get_unchecked(count..);
    }
}

fn all_poll<B: BufferSlice>(res: Poll<usize>, cx: &mut Context, buffer: &mut B) -> Poll<Result<(), ()>> {
    match res {
        Poll::Pending => Poll::Pending,
        Poll::Ready(count) if count >= buffer.len() => {
            debug_assert_eq!(count, buffer.len());
            Poll::Ready(Ok(()))
        },
        Poll::Ready(0) => Poll::Ready(Err(())),
        Poll::Ready(count) => {
            unsafe {
                debug_assert!(count <= buffer.len());
                buffer.resize_from(count);
            }

            // TODO: alternatively tail-recurse into poll
            cx.waker().wake_by_ref();
            Poll::Pending
        },
    }
}
