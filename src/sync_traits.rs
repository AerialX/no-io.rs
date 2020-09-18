use core::fmt;
use core::convert::Infallible;
use super::{AllError, Take};
#[cfg(feature = "ufmt")]
use super::uWriter;

pub(crate) mod prelude {
    pub use super::{Read, ReadExt, Write, WriteExt};
}

// TODO: pull the provided fns out into extension traits instead?

pub trait Read {
    type Error;

    fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error>;

    fn read_exact(&mut self, mut buf: &mut [u8]) -> Result<(), AllError<Self::Error>> {
        // impl stolen from std
        while !buf.is_empty() {
            match self.read(buf) {
                Ok(0) => break,
                Ok(n) => { let tmp = buf; buf = &mut tmp[n..]; }
                // TODO? Err(ref e) if e.kind() == ErrorKind::Interrupted => {}
                Err(e) => return Err(AllError::Io(e)),
            }
        }
        if !buf.is_empty() {
            Err(AllError::UnexpectedEof)
        } else {
            Ok(())
        }
    }

    fn take(self, limit: usize) -> Take<Self> where Self: Sized {
        Take::new(self, limit)
    }
}

impl<T: ?Sized + Read> Read for &'_ mut T {
    type Error = T::Error;

    #[inline]
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
        Read::read(*self, buf)
    }
}

impl Read for &'_ [u8] {
    type Error = Infallible;

    #[inline]
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
        Ok(crate::slice_read(self, buf))
    }
}

pub trait Write {
    type Error;

    fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error>;

    fn flush(&mut self) -> Result<(), Self::Error>;

    fn write_all(&mut self, mut buf: &[u8]) -> Result<(), Self::Error> {
        loop {
            match self.write(buf) {
                Ok(n) if n >= buf.len() => {
                    debug_assert_eq!(n, buf.len());
                    return Ok(())
                },
                #[cfg(debug_assertions)]
                Ok(0) => panic!("Invalid write length"),
                Ok(n) => buf = unsafe { buf.get_unchecked(n..) },
                // TODO? Err(ref e) if e.kind() == ErrorKind::Interrupted => {}
                Err(e) => return Err(e),
            }
        }
    }

    fn write_fmt(&mut self, fmt: fmt::Arguments<'_>) -> Result<(), WriteFmtError<Self::Error>> {
        // impl stolen from std
        struct Adaptor<'a, T: ?Sized + 'a, E> {
            inner: &'a mut T,
            error: Option<E>,
        }

        impl<T: Write + ?Sized> fmt::Write for Adaptor<'_, T, T::Error> {
            fn write_str(&mut self, s: &str) -> fmt::Result {
                match self.inner.write_all(s.as_bytes()) {
                    Ok(()) => Ok(()),
                    Err(e) => {
                        self.error = Some(e);
                        Err(fmt::Error)
                    }
                }
            }
        }

        let mut output = Adaptor { inner: self, error: None::<Self::Error> };
        match fmt::write(&mut output, fmt) {
            Ok(()) => Ok(()),
            Err(..) => Err(match output.error.take() {
                Some(e) => WriteFmtError::Io(e),
                None => WriteFmtError::FormatterError,
            }),
        }
    }

    fn take(self, limit: usize) -> Take<Self> where Self: Sized {
        Take::new(self, limit)
    }

    #[cfg(feature = "ufmt")]
    fn uwriter(self) -> uWriter<Self> where Self: Sized {
        uWriter::new(self)
    }
}

impl<T: ?Sized + Write> Write for &'_ mut T {
    type Error = T::Error;

    #[inline]
    fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
        Write::write(*self, buf)
    }

    #[inline]
    fn flush(&mut self) -> Result<(), Self::Error> {
        Write::flush(*self)
    }
}

impl Write for &'_ mut [u8] {
    type Error = AllError<Infallible>;

    #[inline]
    fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
        crate::slice_write(self, buf)
    }

    #[inline]
    fn flush(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}

impl Write for crate::Sink {
    type Error = Infallible;

    #[inline]
    fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
        Ok(buf.len())
    }

    #[inline]
    fn flush(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}

#[cfg(feature = "ufmt")]
impl ufmt::uWrite for crate::Sink {
    type Error = Infallible;

    #[inline]
    fn write_str(&mut self, _: &str) -> Result<(), Self::Error> {
        Ok(())
    }
}

impl Read for crate::Empty {
    type Error = Infallible;

    #[inline]
    fn read(&mut self, _: &mut [u8]) -> Result<usize, Self::Error> {
        Ok(0)
    }
}

impl Read for crate::Repeat {
    type Error = Infallible;

    #[inline]
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
        for b in &mut *buf {
            *b = self.0
        }
        Ok(buf.len())
    }
}

impl<S: Read> Read for Take<S> {
    type Error = S::Error;

    fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
        let buf = match buf.get_mut(..self.limit) {
            Some(buf) => buf,
            None => buf,
        };
        let res = self.stream.read(buf);
        if let Ok(len) = &res {
            self.limit -= len;
        }
        res
    }
}

impl<S: Write> Write for Take<S> {
    type Error = AllError<S::Error>;

    fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
        if self.limit == 0 && !buf.is_empty() {
            return Err(AllError::UnexpectedEof)
        }

        let buf = match buf.get(..self.limit) {
            Some(buf) => buf,
            None => buf,
        };
        let res = self.stream.write(buf);
        if let Ok(len) = &res {
            self.limit -= len;
        }
        res.map_err(From::from)
    }

    fn flush(&mut self) -> Result<(), Self::Error> {
        self.stream.flush().map_err(From::from)
    }
}

#[cfg(feature = "ufmt")]
impl<W: ?Sized + Write> ufmt::uWrite for uWriter<W> {
    type Error = W::Error;

    fn write_str(&mut self, s: &str) -> Result<(), Self::Error> {
        self.inner.write_all(s.as_bytes())
    }
}

pub trait ReadExt: Read {
    fn copy_to<W: Write>(&mut self, write: W) -> Result<usize, Self::Error> where Self::Error: From<W::Error> {
        copy(self, write)
    }
}

impl<T: ?Sized + Read> ReadExt for T { }

pub trait WriteExt: Write {
    fn copy_from<R: Read>(&mut self, read: R) -> Result<usize, Self::Error> where Self::Error: From<R::Error> {
        copy(read, self)
    }
}

impl<T: ?Sized + Write> WriteExt for T { }

pub fn copy<R: Read, W: Write, E>(mut read: R, mut write: W) -> Result<usize, E> where E: From<R::Error> + From<W::Error> {
    let mut buf = [0u8; 0x10];
    let mut total = 0usize;
    loop {
        let len = match read.read(&mut buf) {
            // TODO? Err(ref e) if e.kind() == ErrorKind::Interrupted => {}
            Err(e) => return Err(e.into()),
            Ok(0) => break,
            Ok(len) => len,
        };
        let buf = unsafe {
            debug_assert!(len <= buf.len());
            buf.get_unchecked(..len)
        };
        write.write_all(&buf)?;
        total = total.saturating_add(len);
    }

    Ok(total)
}

#[derive(Debug, Copy, Clone)]
pub enum WriteFmtError<E> {
    FormatterError,
    Io(E),
}

impl<E: fmt::Display> fmt::Display for WriteFmtError<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WriteFmtError::FormatterError => write!(f, "formatter error"),
            WriteFmtError::Io(err) => err.fmt(f),
        }
    }
}

#[cfg(feature = "std")]
impl<E: std::error::Error + 'static> std::error::Error for WriteFmtError<E> {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            WriteFmtError::FormatterError => None,
            WriteFmtError::Io(err) => Some(err)
        }
    }
}

#[cfg(feature = "std")]
mod std_impl {
    use std::io::{Read, Write, Error, ErrorKind};

    pub struct StdCompat<T: ?Sized>(pub T);

    impl<T: ?Sized> StdCompat<T> {
        #[inline]
        pub fn inner_mut(&mut self) -> &mut T {
            &mut self.0
        }
    }

    impl<T: ?Sized + Read> super::Read for StdCompat<T> {
        type Error = Error;

        #[inline]
        fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
            self.inner_mut().read(buf)
        }
    }

    impl<T: ?Sized + Write> super::Write for StdCompat<T> {
        type Error = Error;

        fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
            match self.inner_mut().write(buf) {
                Ok(0) if buf.is_empty() => Ok(0),
                Ok(n) if n == 0 || n > buf.len() => Err(Error::new(ErrorKind::WriteZero, "Invalid write length")),
                res => res,
            }
        }

        #[inline]
        fn flush(&mut self) -> Result<(), Self::Error> {
            self.inner_mut().flush()
        }
    }

    impl<T: ?Sized + super::Read> Read for StdCompat<T> where
        T::Error: Into<Error>,
    {
        #[inline]
        fn read(&mut self, buf: &mut [u8]) -> Result<usize, Error> {
            self.inner_mut().read(buf)
                .map_err(Into::into)
        }
    }

    impl<T: ?Sized + super::Write> Write for StdCompat<T> where
        T::Error: Into<Error>,
    {
        #[inline]
        fn write(&mut self, buf: &[u8]) -> Result<usize, Error> {
            self.inner_mut().write(buf)
                .map_err(Into::into)
        }

        #[inline]
        fn flush(&mut self) -> Result<(), Error> {
            self.inner_mut().flush()
                .map_err(Into::into)
        }
    }
}

#[cfg(feature = "std")]
pub use std_impl::StdCompat;
