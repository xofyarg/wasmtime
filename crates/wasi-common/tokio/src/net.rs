use io_lifetimes::{AsFd, AsSocketlike};
use std::any::Any;
use std::io;
use tokio::io::unix::AsyncFd;
use wasi_cap_std_sync::net::get_fd_flags;
use wasi_common::{
    file::{FdFlags, FileType, WasiFile},
    Error, ErrorExt,
};

pub struct TcpListener(AsyncFd<cap_std::net::TcpListener>);

impl TcpListener {
    pub(crate) fn from_inner(listener: AsyncFd<cap_std::net::TcpListener>) -> Self {
        TcpListener(listener)
    }
    pub fn from_cap_std(listener: cap_std::net::TcpListener) -> io::Result<Self> {
        Ok(Self::from_inner(AsyncFd::new(listener)?))
    }
}

pub struct TcpStream(AsyncFd<cap_std::net::TcpStream>);

impl TcpStream {
    pub(crate) fn from_inner(stream: AsyncFd<cap_std::net::TcpStream>) -> Self {
        TcpStream(stream)
    }
    pub fn from_cap_std(stream: cap_std::net::TcpStream) -> io::Result<Self> {
        Ok(Self::from_inner(AsyncFd::new(stream)?))
    }
}

#[cfg(unix)]
pub struct UnixListener(AsyncFd<cap_std::os::unix::net::UnixListener>);

#[cfg(unix)]
impl UnixListener {
    pub(crate) fn from_inner(listener: AsyncFd<cap_std::os::unix::net::UnixListener>) -> Self {
        UnixListener(listener)
    }
    pub fn from_cap_std(listener: cap_std::os::unix::net::UnixListener) -> io::Result<Self> {
        Ok(Self::from_inner(AsyncFd::new(listener)?))
    }
}

#[cfg(unix)]
pub struct UnixStream(AsyncFd<cap_std::os::unix::net::UnixStream>);

#[cfg(unix)]
impl UnixStream {
    pub(crate) fn from_inner(stream: AsyncFd<cap_std::os::unix::net::UnixStream>) -> Self {
        UnixStream(stream)
    }
    pub fn from_cap_std(stream: cap_std::os::unix::net::UnixStream) -> io::Result<Self> {
        Ok(Self::from_inner(AsyncFd::new(stream)?))
    }
}

macro_rules! wasi_file_impl {
    ($ty:ty) => {
        #[wiggle::async_trait]
        impl WasiFile for $ty {
            fn as_any(&self) -> &dyn Any {
                self
            }
            #[cfg(unix)]
            fn pollable(&self) -> Option<rustix::fd::BorrowedFd> {
                Some(self.0.get_ref().as_fd())
            }
            async fn get_filetype(&self) -> Result<FileType, Error> {
                Ok(FileType::SocketStream)
            }
            async fn get_fdflags(&self) -> Result<FdFlags, Error> {
                let fdflags = get_fd_flags(&self.0.get_ref())?;
                Ok(fdflags)
            }
            async fn set_fdflags(&mut self, fdflags: FdFlags) -> Result<(), Error> {
                if fdflags == wasi_common::file::FdFlags::NONBLOCK {
                    self.0.get_ref().set_nonblocking(true)?;
                } else if fdflags.is_empty() {
                    self.0.get_ref().set_nonblocking(false)?;
                } else {
                    return Err(
                        Error::invalid_argument().context("cannot set anything else than NONBLOCK")
                    );
                }
                Ok(())
            }
            async fn read_vectored<'a>(
                &self,
                bufs: &mut [io::IoSliceMut<'a>],
            ) -> Result<u64, Error> {
                use std::io::Read;
                let n = Read::read_vectored(
                    &mut &*self
                        .0
                        .get_ref()
                        .as_socketlike_view::<std::os::unix::net::UnixStream>(),
                    bufs,
                )?;
                Ok(n.try_into()?)
            }
            async fn write_vectored<'a>(&self, bufs: &[io::IoSlice<'a>]) -> Result<u64, Error> {
                use std::io::Write;
                let n = Write::write_vectored(
                    &mut &*self
                        .0
                        .get_ref()
                        .as_socketlike_view::<std::os::unix::net::UnixStream>(),
                    bufs,
                )?;
                Ok(n.try_into()?)
            }
            fn num_ready_bytes(&self) -> Result<u64, Error> {
                Ok(1)
            }

            #[cfg(not(windows))]
            async fn readable(&self) -> Result<(), Error> {
                let mut guard = self.0.readable().await?;
                guard.clear_ready();
                Ok(())
            }

            #[cfg(not(windows))]
            async fn writable(&self) -> Result<(), Error> {
                let mut guard = self.0.writable().await?;
                guard.clear_ready();
                Ok(())
            }
        }
        #[cfg(windows)]
        impl AsRawHandleOrSocket for $ty {
            #[inline]
            fn as_raw_handle_or_socket(&self) -> RawHandleOrSocket {
                self.0.borrow().as_raw_handle_or_socket()
            }
        }
    };
}

wasi_file_impl!(TcpListener);
wasi_file_impl!(TcpStream);
#[cfg(unix)]
wasi_file_impl!(UnixListener);
#[cfg(unix)]
wasi_file_impl!(UnixStream);
