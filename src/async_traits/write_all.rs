use core::future::Future;
use core::task::{Context, Poll};
use core::pin::Pin;
use super::all_poll_write;

#[inline]
pub fn write_all<'a, 'b, T>(this: Pin<&'a mut T>, buffer: &'b [u8]) -> WriteAll<'a, 'b, T> {
    WriteAll {
        this,
        buffer,
    }
}

pub struct WriteAll<'a, 'b, T> {
    this: Pin<&'a mut T>,
    buffer: &'b [u8],
}

impl<'a, 'b, T: super::AsyncWrite> Future for WriteAll<'a, 'b, T> {
    type Output = Result<(), T::Error>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        let s = self.as_mut().get_mut();
        let this = s.this.as_mut();
        let res = this.poll_write(cx, s.buffer)?;
        all_poll_write(res, cx, &mut self.buffer).map(Ok)
    }
}
