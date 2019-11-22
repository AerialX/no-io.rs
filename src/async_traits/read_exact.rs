use core::future::Future;
use core::task::{Context, Poll};
use core::pin::Pin;
use core::mem::replace;
use crate::AllError;

pub fn read_exact<'a, 'b, T>(this: Pin<&'a mut T>, buffer: &'b mut [u8]) -> ReadExact<'a, 'b, T> {
    ReadExact {
        this,
        buffer,
    }
}

pub struct ReadExact<'a, 'b, T> {
    this: Pin<&'a mut T>,
    buffer: &'b mut [u8],
}

impl<'a, 'b, T: super::AsyncRead> Future for ReadExact<'a, 'b, T> {
    type Output = Result<(), AllError<T::Error>>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        let s = self.as_mut().get_mut();
        let this = s.this.as_mut();
        match this.poll_read(cx, s.buffer) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(Err(e)) => Poll::Ready(Err(AllError::Io(e))),
            Poll::Ready(Ok(count)) if count >= s.buffer.len() => {
                debug_assert_eq!(count, s.buffer.len());
                Poll::Ready(Ok(()))
            },
            Poll::Ready(Ok(0)) => Poll::Ready(Err(AllError::UnexpectedEof)),
            Poll::Ready(Ok(count)) => {
                debug_assert!(count <= s.buffer.len());
                let buffer = replace(&mut self.buffer, &mut []);
                self.buffer = unsafe { buffer.get_unchecked_mut(count..) };
                self.poll(cx) // TODO: alternatively: { cx.waker().wake_by_ref(); Pending }
            },
        }
    }
}
