use core::future::Future;
use core::task::{Context, Poll};
use core::pin::Pin;
use super::all_poll_write;

pub struct AsyncWriteAll<'a, 'b, T: ?Sized> {
    pub(crate) this: Pin<&'a mut T>,
    pub(crate) buffer: &'b [u8],
}

impl<'a, 'b, T: ?Sized + super::AsyncWrite> Future for AsyncWriteAll<'a, 'b, T> {
    type Output = Result<(), T::Error>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        let s = self.as_mut().get_mut();
        let this = s.this.as_mut();
        let res = this.poll_write(cx, s.buffer)?;
        all_poll_write(res, cx, &mut self.buffer).map(Ok)
    }
}
