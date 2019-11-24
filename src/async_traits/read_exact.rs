use core::future::Future;
use core::task::{Context, Poll};
use core::pin::Pin;
use crate::AllError;
use super::all_poll;

pub struct AsyncReadExact<'a, 'b, T: ?Sized> {
    pub(crate) this: Pin<&'a mut T>,
    pub(crate) buffer: &'b mut [u8],
}

impl<'a, 'b, T: ?Sized + super::AsyncRead> Future for AsyncReadExact<'a, 'b, T> {
    type Output = Result<(), AllError<T::Error>>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        let s = self.as_mut().get_mut();
        let this = s.this.as_mut();
        let res = this.poll_read(cx, s.buffer)?;
        all_poll(res, cx, &mut self.buffer).map_err(|_| AllError::UnexpectedEof)
    }
}
