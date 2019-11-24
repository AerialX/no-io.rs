pub struct HashStream<H, S> {
    pub hasher: H,
    pub stream: S,
}

impl<H, S> HashStream<H, S> {
    pub fn new(hasher: H, stream: S) -> Self {
        Self {
            hasher,
            stream,
        }
    }

    pub fn into_inner(self) -> (H, S) {
        (self.hasher, self.stream)
    }
}

#[cfg(feature = "sync")]
mod sync_impl {
    use core::hash::Hasher;
    use super::HashStream;

    impl<H: Hasher, S: crate::Read> crate::Read for HashStream<H, S> {
        type Error = S::Error;

        fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
            let res = self.stream.read(buf);
            if let Ok(len) = &res {
                self.hasher.write(unsafe { buf.get_unchecked(..*len) });
            }
            res
        }
    }

    impl<H: Hasher, S: crate::Write> crate::Write for HashStream<H, S> {
        type Error = S::Error;

        fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
            let res = self.stream.write(buf);
            if let Ok(len) = &res {
                self.hasher.write(unsafe { buf.get_unchecked(..*len) })
            }
            res
        }

        #[inline]
        fn flush(&mut self) -> Result<(), Self::Error> {
            self.stream.flush()
        }
    }
}

#[cfg(feature = "async")]
mod async_impl {
    use core::task::{Context, Poll};
    use core::pin::Pin;
    use core::hash::Hasher;
    use super::HashStream;

    impl<H: Hasher, S: crate::AsyncRead> crate::AsyncRead for HashStream<H, S> {
        type Error = S::Error;

        fn poll_read(self: Pin<&mut Self>, cx: &mut Context, buf: &mut [u8]) -> Poll<Result<usize, Self::Error>> {
            let this = unsafe { self.get_unchecked_mut() };
            let stream = unsafe { Pin::new_unchecked(&mut this.stream) };
            let res = stream.poll_read(cx, buf);
            if let Poll::Ready(Ok(len)) = &res {
                this.hasher.write(unsafe { buf.get_unchecked(..*len) });
            }

            res
        }
    }

    impl<H: Hasher, S: crate::AsyncWrite> crate::AsyncWrite for HashStream<H, S> {
        type Error = S::Error;

        fn poll_write(self: Pin<&mut Self>, cx: &mut Context, buf: &[u8]) -> Poll<Result<usize, Self::Error>> {
            let this = unsafe { self.get_unchecked_mut() };
            let stream = unsafe { Pin::new_unchecked(&mut this.stream) };
            let res = stream.poll_write(cx, buf);
            if let Poll::Ready(Ok(len)) = &res {
                    this.hasher.write(unsafe { buf.get_unchecked(..*len) })
            }
            res
        }

        #[inline]
        fn poll_flush(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Result<(), Self::Error>> {
            let this = unsafe { self.get_unchecked_mut() };
            let stream = unsafe { Pin::new_unchecked(&mut this.stream) };
            stream.poll_flush(cx)
        }

        #[inline]
        fn poll_close(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Result<(), Self::Error>> {
            let this = unsafe { self.get_unchecked_mut() };
            let stream = unsafe { Pin::new_unchecked(&mut this.stream) };
            stream.poll_flush(cx)
        }
    }
}
