mod re_export;

pub use re_export::*;

use futures_util::{Stream, StreamExt, TryStream, TryStreamExt};
use std::{
    error::Error as StdError,
    fmt::Debug,
    pin::Pin,
    task::{Context, Poll},
};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tokio_native_tls::{TlsAcceptor, TlsStream};
use tonic::transport::server::Connected;

pub type Error = Box<dyn StdError + Send + Sync + 'static>;

pub fn incoming<S>(
    incoming: S,
    acceptor: TlsAcceptor,
) -> impl Stream<Item = Result<TlsStreamWrapper<S::Ok>, Error>>
where
    S: TryStream + Unpin,
    S::Ok: AsyncRead + AsyncWrite + Unpin + Send + Sync + 'static,
    S::Error: StdError + Send + Sync + 'static,
{
    incoming.into_stream().then(move |stream| {
        // TODO: Wrap into Arc?
        let acceptor = acceptor.clone();
        async move { Ok(TlsStreamWrapper(acceptor.accept(stream?).await?)) }
    })
}

#[derive(Debug)]
pub struct TlsStreamWrapper<S>(TlsStream<S>);

impl<S> Connected for TlsStreamWrapper<S>
where
    S: Connected + AsyncRead + AsyncWrite + Unpin,
{
    type ConnectInfo = <S as Connected>::ConnectInfo;

    fn connect_info(&self) -> Self::ConnectInfo {
        self.0.get_ref().get_ref().get_ref().connect_info()
    }
}

impl<S> AsyncRead for TlsStreamWrapper<S>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        Pin::new(&mut self.0).poll_read(cx, buf)
    }
}

impl<S> AsyncWrite for TlsStreamWrapper<S>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize, std::io::Error>> {
        Pin::new(&mut self.0).poll_write(cx, buf)
    }

    fn poll_flush(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Result<(), std::io::Error>> {
        Pin::new(&mut self.0).poll_flush(cx)
    }

    fn poll_shutdown(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Result<(), std::io::Error>> {
        Pin::new(&mut self.0).poll_shutdown(cx)
    }
}
