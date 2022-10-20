#[allow(clippy::derive_partial_eq_without_eq)]
mod grpc;

use crate::grpc::echo::echo_server::EchoServer;
use crate::grpc::echo::{echo_server::Echo, EchoRequest, EchoResponse};
use native_tls::Identity;
use std::net::SocketAddr;
use tokio::net::TcpListener;
use tokio_native_tls::TlsAcceptor;
use tokio_stream::wrappers::TcpListenerStream;
use tonic::transport::Server;
use tonic::{async_trait, Request as TonicRequest, Response as TonicResponse, Status};
use tonic_native_tls::{native_tls, tokio_native_tls};

#[derive(Default)]
pub struct MyEcho;

#[async_trait]
impl Echo for MyEcho {
    async fn ping(
        &self,
        request: TonicRequest<EchoRequest>,
    ) -> Result<TonicResponse<EchoResponse>, Status> {
        let message = EchoResponse {
            message: request.into_inner().message,
        };

        Ok(TonicResponse::new(message))
    }
}

const PEM: &[u8] = include_bytes!("../certs/tls.cert");
const KEY: &[u8] = include_bytes!("../certs/tls.key");

#[tokio::main]
async fn main() {
    let identity = Identity::from_pkcs8(PEM, KEY).expect("Can't build Identity.");
    let acceptor =
        TlsAcceptor::from(native_tls::TlsAcceptor::new(identity).expect("Can't build TlsAcceptor"));

    let addr = "[::0]:50051"
        .parse::<SocketAddr>()
        .expect("The Word was fallen.");
    let listener = TcpListener::bind(addr)
        .await
        .expect("Can't build listener.");
    let stream = TcpListenerStream::new(listener);
    let incoming = tonic_native_tls::incoming(stream, acceptor);

    let echo = MyEcho::default();
    let grpc_reflection = tonic_reflection::server::Builder::configure()
        .register_encoded_file_descriptor_set(include_bytes!("../src/grpc/description.bin"))
        .build()
        .expect("Can't build reflection service.");

    println!("EchoServer listening on {addr}");

    Server::builder()
        .add_service(EchoServer::new(echo))
        .add_service(grpc_reflection)
        .serve_with_incoming(incoming)
        .await
        .expect("Server serve error.")
}
