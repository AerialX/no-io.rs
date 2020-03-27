use core::future::Future;
use core::task::{Context, Poll};
use core::pin::Pin;

pub struct AsyncFlush<'a, T: ?Sized> {
    this: Pin<&'a mut T>,
}

impl<'a, T: ?Sized> AsyncFlush<'a, T> {
    pub fn new(this: Pin<&'a mut T>) -> Self {
        Self {
            this,
        }
    }
}

impl<'a, T: ?Sized + super::AsyncWrite> Future for AsyncFlush<'a, T> {
    type Output = Result<(), T::Error>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        let s = self.as_mut().get_mut();
        let this = s.this.as_mut();
        this.poll_flush(cx)
    }
}
