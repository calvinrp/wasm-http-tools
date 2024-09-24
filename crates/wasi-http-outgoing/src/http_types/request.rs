use super::Method;
use url::Url;
use wasi::{
    http::{
        outgoing_handler::OutgoingRequest,
        types::{Headers as WasiHeaders, Scheme, OutgoingBody},
    },
    io::streams::{InputStream, StreamError},
};

/// An HTTP request
#[derive(Debug)]
pub struct Request {
    method: Method,
    url: Url,
    headers: WasiHeaders,

    // IMPORTANT: the order of these fields here matters. `outgoing_body` must
    // be dropped before `body_stream`.
    body_stream: OutputStream,
    _outgoing_body: OutgoingBody,
}

impl Request {
    /// Create a new HTTP request to send off to the client.
    pub fn new(method: Method, url: Url) -> Self {
        Self {
            method,
            url,
            headers: WasiHeaders::new(),
        }
    }
}

impl From<Request> for OutgoingRequest {
    fn from(req: Request) -> Self {
        // Copy over the HTTP headers
        let wasi_req = OutgoingRequest::new(req.headers);

        // Set the HTTP method
        wasi_req.set_method(&req.method.into()).unwrap();

        // Set the url scheme
        let scheme = match req.url.scheme() {
            "http" => Scheme::Http,
            "https" => Scheme::Https,
            other => Scheme::Other(other.to_owned()),
        };
        wasi_req.set_scheme(Some(&scheme)).unwrap();

        // Set the url path + query string
        let path = match req.url.query() {
            Some(query) => format!("{}?{query}", req.url.path()),
            None => req.url.path().to_owned(),
        };
        wasi_req.set_path_with_query(Some(&path)).unwrap();

        // Not sure why we also have to set the authority, but sure we can do
        // that too!
        wasi_req.set_authority(Some(req.url.authority())).unwrap();

        // All done; request is ready for send-off
        wasi_req
    }
}
