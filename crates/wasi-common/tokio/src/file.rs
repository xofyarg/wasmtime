#[cfg(windows)]
use io_extras::os::windows::{AsRawHandleOrSocket, RawHandleOrSocket};
#[cfg(not(windows))]
use io_lifetimes::AsFd;
use std::any::Any;
use std::borrow::Borrow;
use std::io;
use wasi_common::{
    file::{Advice, FdFlags, FileType, Filestat, WasiFile},
    Error,
};

pub struct File(wasi_cap_std_sync::file::File);

impl File {
    pub(crate) fn from_inner(file: wasi_cap_std_sync::file::File) -> Self {
        File(file)
    }
    pub fn from_cap_std(file: cap_std::fs::File) -> Self {
        Self::from_inner(wasi_cap_std_sync::file::File::from_cap_std(file))
    }
}

pub struct Stdin(wasi_cap_std_sync::stdio::Stdin);

pub fn stdin() -> Stdin {
    Stdin(wasi_cap_std_sync::stdio::stdin())
}

pub struct Stdout(wasi_cap_std_sync::stdio::Stdout);

pub fn stdout() -> Stdout {
    Stdout(wasi_cap_std_sync::stdio::stdout())
}

pub struct Stderr(wasi_cap_std_sync::stdio::Stderr);

pub fn stderr() -> Stderr {
    Stderr(wasi_cap_std_sync::stdio::stderr())
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
                Some(self.0.as_fd())
            }
            #[cfg(windows)]
            fn pollable(&self) -> Option<io_extras::os::windows::RawHandleOrSocket> {
                Some(self.0.as_raw_handle_or_socket())
            }
            async fn datasync(&self) -> Result<(), Error> {
                self.0.datasync().await
            }
            async fn sync(&self) -> Result<(), Error> {
                self.0.sync().await
            }
            async fn get_filetype(&self) -> Result<FileType, Error> {
                self.0.get_filetype().await
            }
            async fn get_fdflags(&self) -> Result<FdFlags, Error> {
                self.0.get_fdflags().await
            }
            async fn set_fdflags(&mut self, fdflags: FdFlags) -> Result<(), Error> {
                self.0.set_fdflags(fdflags).await
            }
            async fn get_filestat(&self) -> Result<Filestat, Error> {
                self.0.get_filestat().await
            }
            async fn set_filestat_size(&self, size: u64) -> Result<(), Error> {
                self.0.set_filestat_size(size).await
            }
            async fn advise(&self, offset: u64, len: u64, advice: Advice) -> Result<(), Error> {
                self.0.advise(offset, len, advice).await
            }
            async fn read_vectored<'a>(
                &self,
                bufs: &mut [io::IoSliceMut<'a>],
            ) -> Result<u64, Error> {
                self.0.read_vectored(bufs).await
            }
            async fn read_vectored_at<'a>(
                &self,
                bufs: &mut [io::IoSliceMut<'a>],
                offset: u64,
            ) -> Result<u64, Error> {
                self.0.read_vectored_at(bufs, offset).await
            }
            async fn write_vectored<'a>(&self, bufs: &[io::IoSlice<'a>]) -> Result<u64, Error> {
                self.0.write_vectored(bufs).await
            }
            async fn write_vectored_at<'a>(
                &self,
                bufs: &[io::IoSlice<'a>],
                offset: u64,
            ) -> Result<u64, Error> {
                self.0.write_vectored_at(bufs, offset).await
            }
            async fn seek(&self, pos: std::io::SeekFrom) -> Result<u64, Error> {
                self.0.seek(pos).await
            }
            async fn peek(&self, buf: &mut [u8]) -> Result<u64, Error> {
                self.0.peek(buf).await
            }
            async fn set_times(
                &self,
                atime: Option<wasi_common::SystemTimeSpec>,
                mtime: Option<wasi_common::SystemTimeSpec>,
            ) -> Result<(), Error> {
                self.0.set_times(atime, mtime).await
            }
            fn num_ready_bytes(&self) -> Result<u64, Error> {
                self.0.num_ready_bytes()
            }
            fn isatty(&self) -> bool {
                self.0.isatty()
            }

            #[cfg(not(windows))]
            async fn readable(&self) -> Result<(), Error> {
                // The Inner impls OwnsRaw, which asserts exclusive use of the handle by the owned object.
                // AsyncFd needs to wrap an owned `impl std::os::unix::io::AsRawFd`. Rather than introduce
                // mutability to let it own the `Inner`, we are depending on the `&mut self` bound on this
                // async method to ensure this is the only Future which can access the RawFd during the
                // lifetime of the AsyncFd.
                use std::os::unix::io::AsRawFd;
                use tokio::io::{unix::AsyncFd, Interest};
                let rawfd = self.0.borrow().as_fd().as_raw_fd();
                match AsyncFd::with_interest(rawfd, Interest::READABLE) {
                    Ok(asyncfd) => {
                        let _ = asyncfd.readable().await?;
                        Ok(())
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => {
                        // if e is EPERM, this file isnt supported by epoll because it is immediately
                        // available for reading:
                        Ok(())
                    }
                    Err(e) => Err(e.into()),
                }
            }

            #[cfg(not(windows))]
            async fn writable(&self) -> Result<(), Error> {
                // The Inner impls OwnsRaw, which asserts exclusive use of the handle by the owned object.
                // AsyncFd needs to wrap an owned `impl std::os::unix::io::AsRawFd`. Rather than introduce
                // mutability to let it own the `Inner`, we are depending on the `&mut self` bound on this
                // async method to ensure this is the only Future which can access the RawFd during the
                // lifetime of the AsyncFd.
                use std::os::unix::io::AsRawFd;
                use tokio::io::{unix::AsyncFd, Interest};
                let rawfd = self.0.borrow().as_fd().as_raw_fd();
                match AsyncFd::with_interest(rawfd, Interest::WRITABLE) {
                    Ok(asyncfd) => {
                        let _ = asyncfd.writable().await?;
                        Ok(())
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => {
                        // if e is EPERM, this file isnt supported by epoll because it is immediately
                        // available for writing:
                        Ok(())
                    }
                    Err(e) => Err(e.into()),
                }
            }

            async fn sock_accept(&self, fdflags: FdFlags) -> Result<Box<dyn WasiFile>, Error> {
                self.0.sock_accept(fdflags).await
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

wasi_file_impl!(File);
wasi_file_impl!(Stdin);
wasi_file_impl!(Stdout);
wasi_file_impl!(Stderr);
