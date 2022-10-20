#[allow(clippy::derive_partial_eq_without_eq)]
mod grpc;

use crate::grpc::echo::{echo_client::EchoClient, EchoRequest};
use hyper::client::HttpConnector;
use hyper::Client;
use hyper_tls::{native_tls, HttpsConnector};
use tonic::Request;

#[tokio::main]
async fn main() {
    let mut http = HttpConnector::new();
    http.enforce_http(false);

    let tls_connector = native_tls::TlsConnector::builder()
        .danger_accept_invalid_certs(true)
        .build()
        .unwrap()
        .into();

    let https = HttpsConnector::from((http, tls_connector));

    let hyper = Client::builder().http2_only(true).build(https);
    let mut client = EchoClient::with_origin(hyper, "https://[::1]:50051".parse().unwrap());

    let request = Request::new(EchoRequest {
        message: "Hello from Jindřich from Skalica!".to_string(),
    });

    let response = client.ping(request).await.unwrap().into_inner();
    println!("response: `{response:?}`.");
}
