use core::fmt;
use super::AllError;

pub enum WriteFmtError<E> {
    FormatterError,
    // TODO: embed AllError::WriteZero here?
    Io(E),
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
}

impl<T: Read> Read for &'_ mut T {
    type Error = T::Error;

    fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
        Read::read(*self, buf)
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
}

// TODO: figure out how to impl std::io traits ughh why is rust so bad

#[cfg(feature = "std")]
mod std_impl {
    use std::io::{Read, Write, Error, ErrorKind};

    pub struct StdCompat<T: ?Sized>(pub T);

    impl<T: ?Sized> StdCompat<T> {
        pub fn inner_mut(&mut self) -> &mut T {
            &mut self.0
        }
    }

    impl<T: ?Sized + Read> super::Read for StdCompat<T> {
        type Error = Error;

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

        fn flush(&mut self) -> Result<(), Self::Error> {
            self.inner_mut().flush()
        }
    }
}

#[cfg(feature = "std")]
pub use std_impl::StdCompat;
