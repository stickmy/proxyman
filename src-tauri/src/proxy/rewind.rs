use bytes::Bytes;
use std::{
    io,
    marker::Unpin,
    pin::Pin,
    task::{Context, Poll},
};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};

pub(crate) struct Rewind<T> {
    prefix: Option<Bytes>,
    inner: T,
}

impl<T> Rewind<T> {
    #[allow(dead_code)]
    pub(crate) fn new(io: T) -> Self {
        Rewind {
            prefix: None,
            inner: io,
        }
    }

    pub(crate) fn new_buffered(io: T, prefix: Bytes) -> Self {
        Rewind {
            prefix: Some(prefix),
            inner: io,
        }
    }

    #[allow(dead_code)]
    pub(crate) fn rewind(&mut self, bs: Bytes) {
        debug_assert!(self.prefix.is_none());
        self.prefix = Some(bs);
    }

    #[allow(dead_code)]
    pub(crate) fn into_inner(self) -> (T, Bytes) {
        (self.inner, self.prefix.unwrap_or_else(Bytes::new))
    }
}

impl<T: AsyncRead + Unpin> AsyncRead for Rewind<T> {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context,
        buf: &mut ReadBuf,
    ) -> Poll<io::Result<()>> {
        if let Some(mut prefix) = self.prefix.take() {
            // If the Vec was empty, we'll want to avoid returning zero bytes. Tokio considers
            // a return of zero bytes as stream having ended, which is not what is happening here.

            if prefix.is_empty() {
                self.prefix = None;
            } else if prefix.len() <= buf.remaining() {
                buf.put_slice(&prefix);
                self.prefix = None;
                return Poll::Ready(Ok(()));
            } else {
                let mut taken = prefix.split_off(buf.remaining());
                std::mem::swap(&mut taken, &mut prefix);
                buf.put_slice(&taken);
                return Poll::Ready(Ok(()));
            }
        }

        Pin::new(&mut self.inner).poll_read(cx, buf)
    }
}

impl<T: AsyncWrite + Unpin> AsyncWrite for Rewind<T> {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        Pin::new(&mut self.inner).poll_write(cx, buf)
    }

    fn poll_write_vectored(
        mut self: Pin<&mut Self>,
        cx: &mut Context,
        bufs: &[io::IoSlice],
    ) -> Poll<io::Result<usize>> {
        Pin::new(&mut self.inner).poll_write_vectored(cx, bufs)
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<io::Result<()>> {
        Pin::new(&mut self.inner).poll_flush(cx)
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<io::Result<()>> {
        Pin::new(&mut self.inner).poll_shutdown(cx)
    }

    fn is_write_vectored(&self) -> bool {
        self.inner.is_write_vectored()
    }
}
