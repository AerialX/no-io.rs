use core::future::Future;
use core::task::{Context, Poll};
use core::pin::Pin;
use crate::AllError;

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
    type Output = Result<(), AllError<T::Error>>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        let s = self.as_mut().get_mut();
        let this = s.this.as_mut();
        match this.poll_write(cx, s.buffer) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(Err(e)) => Poll::Ready(Err(AllError::Io(e))),
            Poll::Ready(Ok(count)) if count >= s.buffer.len() => {
                debug_assert_eq!(count, s.buffer.len());
                Poll::Ready(Ok(()))
            },
            Poll::Ready(Ok(0)) => Poll::Ready(Err(AllError::UnexpectedEof)),
            Poll::Ready(Ok(count)) => {
                debug_assert!(count <= s.buffer.len());
                self.buffer = unsafe { self.buffer.get_unchecked(count..) };
                self.poll(cx) // TODO: alternatively: { cx.waker().wake_by_ref(); Pending }
            },
        }
    }
}
