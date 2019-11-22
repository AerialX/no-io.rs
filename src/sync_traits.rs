use core::fmt;
#[cfg(feature = "std")]
use std::io;
use super::AllError;

pub enum WriteFmtError<E> {
    FormatterError,
    // TODO: embed AllError::WriteZero here?
    Io(AllError<E>),
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

#[cfg(feature = "std")]
impl<T: io::Read> Read for T {
    type Error = io::Error;

    fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
        io::Read::read(self, buf)
    }
}

pub trait Write {
    type Error;

    fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error>;

    fn flush(&mut self) -> Result<(), Self::Error>;

    fn write_all(&mut self, mut buf: &[u8]) -> Result<(), AllError<Self::Error>> {
        // impl stolen from std
        while !buf.is_empty() {
            match self.write(buf) {
                Ok(0) => return Err(AllError::UnexpectedEof),
                Ok(n) => buf = &buf[n..],
                // TODO? Err(ref e) if e.kind() == ErrorKind::Interrupted => {}
                Err(e) => return Err(AllError::Io(e)),
            }
        }
        Ok(())
    }

    fn write_fmt(&mut self, fmt: fmt::Arguments<'_>) -> Result<(), WriteFmtError<Self::Error>> {
        // impl stolen from std
        struct Adaptor<'a, T: ?Sized + 'a, E> {
            inner: &'a mut T,
            error: Option<AllError<E>>,
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

        let mut output = Adaptor { inner: self, error: None::<AllError<Self::Error>> };
        match fmt::write(&mut output, fmt) {
            Ok(()) => Ok(()),
            Err(..) => Err(match output.error.take() {
                Some(e) => WriteFmtError::Io(e),
                None => WriteFmtError::FormatterError,
            }),
        }
    }
}

#[cfg(feature = "std")]
impl<T: io::Write> Write for T {
    type Error = io::Error;

    fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
        io::Write::write(self, buf)
    }

    fn flush(&mut self) -> Result<(), Self::Error> {
        io::Write::flush(self)
    }
}
