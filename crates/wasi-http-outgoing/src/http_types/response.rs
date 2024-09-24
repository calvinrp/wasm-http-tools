use wasi::{
    http::types::{IncomingBody, IncomingResponse},
    io::streams::{InputStream, StreamError},
};
use wasi_async_runtime::Reactor;

use crate::Headers;

/// Stream 2kb chunks at a time
const CHUNK_SIZE: u64 = 2048;

/// An HTTP response
#[derive(Debug)]
pub struct Response {
    bytes_read: u64,
    content_length: Option<u64>,
    headers: Headers,
    reactor: Reactor,

    // IMPORTANT: the order of these fields here matters. `incoming_body` must
    // be dropped before `body_stream`.
    body_stream: InputStream,
    _incoming_body: IncomingBody,
}

impl Response {
    pub(crate) fn try_from_incoming(
        incoming: IncomingResponse,
        reactor: Reactor,
    ) -> crate::Result<Self> {
        let headers: Headers = incoming.headers().into();

        let content_length = match headers.0.get("content-length") {
            Some(vals) => match vals.first() {
                Some(v) => match std::str::from_utf8(v).ok() {
                    Some(s) => s.parse().ok(),
                    None => None,
                },
                None => None,
            },
            None => None,
        };

        // `body_stream` is a child of `incoming_body` which means we cannot
        // drop the parent before we drop the child
        let incoming_body = incoming
            .consume()
            .expect("cannot call `consume` twice on incoming response");
        let body_stream = incoming_body
            .stream()
            .expect("cannot call `stream` twice on an incoming body");

        Ok(Self {
            bytes_read: 0,
            headers,
            content_length,
            body_stream,
            _incoming_body: incoming_body,
            reactor,
        })
    }

    /// Get the HTTP headers from the impl
    pub fn headers(&self) -> &Headers {
        &self.headers
    }

    /// Mutably get the HTTP headers from the impl
    pub fn headers_mut(&mut self) -> &mut Headers {
        &mut self.headers
    }
}

impl async_iterator::Iterator for Response {
    type Item = Result<Vec<u8>, StreamError>;

    async fn next(&mut self) -> Option<Self::Item> {
        // Calculate how many bytes we can read
        let remaining = self.content_length.unwrap_or(CHUNK_SIZE) - self.bytes_read;

        let len = remaining.min(CHUNK_SIZE);
        if len == 0 {
            return None;
        }

        // Wait for an event to be ready
        let pollable = self.body_stream.subscribe();
        self.reactor.wait_for(pollable).await;

        // Read the bytes from the body stream
        let buf = self.body_stream.read(len);
        self.bytes_read += len;
        Some(buf)
    }
}
