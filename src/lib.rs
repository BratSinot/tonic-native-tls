mod re_export;

pub use re_export::*;

use async_stream::try_stream;
use futures_util::{Stream, StreamExt, TryStream, TryStreamExt};
use std::{
    error::Error as StdError,
    fmt::Debug,
    future::ready,
    pin::Pin,
    task::{Context, Poll},
};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tokio_native_tls::{TlsAcceptor, TlsStream};

pub type Error = Box<dyn StdError + Send + Sync + 'static>;

pub fn incoming<S>(
    mut incoming: S,
    acceptor: TlsAcceptor,
) -> impl Stream<Item = Result<TlsStreamWrapper<S::Ok>, Error>>
where
    S: TryStream + Unpin,
    S::Ok: AsyncRead + AsyncWrite + Unpin + Send + Sync + 'static,
    S::Error: StdError + Send + Sync + 'static,
{
    try_stream! {
        while let Some(stream) = incoming.try_next().await? {
            yield TlsStreamWrapper(acceptor.accept(stream).await?);
        }
    }
    .filter(|tls_stream| {
        let ret = if let Err(error) = tls_stream {
            #[cfg(feature = "tracing")]
            tracing::error!("Got error on incoming: `{error}`.");
            false
        } else {
            true
        };

        ready(ret)
    })
}

#[derive(Debug)]
pub struct TlsStreamWrapper<S>(TlsStream<S>);

#[cfg(feature = "axum")]
impl axum::extract::connect_info::Connected<&TlsStreamWrapper<tokio::net::TcpStream>>
    for std::net::SocketAddr
{
    fn connect_info(target: &TlsStreamWrapper<tokio::net::TcpStream>) -> Self {
        use std::net::{IpAddr, Ipv4Addr, SocketAddr};

        target
            .0
            .get_ref()
            .get_ref()
            .get_ref()
            .peer_addr()
            .unwrap_or(SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), 0))
    }
}

#[cfg(feature = "tonic")]
impl<S> tonic::transport::server::Connected for TlsStreamWrapper<S>
where
    S: tonic::transport::server::Connected + AsyncRead + AsyncWrite + Unpin,
{
    type ConnectInfo = <S as tonic::transport::server::Connected>::ConnectInfo;

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
