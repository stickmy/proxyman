use crate::error::EndpointError;
use async_compression::tokio::bufread::{BrotliDecoder, GzipDecoder, ZlibDecoder, ZstdDecoder};
use bstr::ByteSlice;
use bytes::Bytes;
use futures::Stream;
use http::{
    header::{CONTENT_ENCODING, CONTENT_LENGTH},
    HeaderMap, HeaderValue,
};
use hyper::{Body, Error as HyperError, Request, Response};
use std::{
    io,
    io::Error as IoError,
    pin::Pin,
    task::{Context, Poll},
};
use tokio::io::{AsyncBufRead, AsyncRead, BufReader};
use tokio_util::io::{ReaderStream, StreamReader};

pub struct IoStream<T: Stream<Item = Result<Bytes, HyperError>> + Unpin>(pub T);

impl<T: Stream<Item = Result<Bytes, HyperError>> + Unpin> Stream for IoStream<T> {
    type Item = Result<Bytes, IoError>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match futures::ready!(Pin::new(&mut self.0).poll_next(cx)) {
            Some(Ok(chunk)) => Poll::Ready(Some(Ok(chunk))),
            Some(Err(err)) => Poll::Ready(Some(Err(IoError::new(io::ErrorKind::Other, err)))),
            None => Poll::Ready(None),
        }
    }
}

enum Decoder {
    Body(Body),
    Decoder(Box<dyn AsyncRead + Send + Unpin>),
}

impl Decoder {
    pub fn decode(self, encoding: &[u8]) -> Result<Self, EndpointError> {
        if encoding == b"identity" {
            return Ok(self);
        }

        let reader: Box<dyn AsyncBufRead + Send + Unpin> = match self {
            Self::Body(body) => Box::new(StreamReader::new(IoStream(body))),
            Self::Decoder(decoder) => Box::new(BufReader::new(decoder)),
        };

        let decoder: Box<dyn AsyncRead + Send + Unpin> = match encoding {
            b"gzip" | b"x-gzip" => Box::new(GzipDecoder::new(reader)),
            b"deflate" => Box::new(ZlibDecoder::new(reader)),
            b"br" => Box::new(BrotliDecoder::new(reader)),
            b"zstd" => Box::new(ZstdDecoder::new(reader)),
            _ => {
                return Err(EndpointError::Decoder {
                    scenario: "unknown decoding",
                })
            }
        };

        Ok(Self::Decoder(decoder))
    }
}

impl From<Decoder> for Body {
    fn from(value: Decoder) -> Self {
        match value {
            Decoder::Body(body) => body,
            Decoder::Decoder(decoder) => Body::wrap_stream(ReaderStream::new(decoder)),
        }
    }
}

// encoding1,encoding2 -> ["encoding1", "encoding2"]
fn parse_encodings(headers: &HeaderMap<HeaderValue>) -> impl Iterator<Item = &[u8]> {
    headers
        .get_all(CONTENT_ENCODING)
        .iter()
        .rev()
        .flat_map(|val| val.as_bytes().rsplit_str(b",").map(|v| v.trim()))
}

fn decode_body<'a>(
    encodings: impl IntoIterator<Item = &'a [u8]>,
    body: Body,
) -> Result<Body, EndpointError> {
    let mut decoder = Decoder::Body(body);

    for encoding in encodings {
        decoder = decoder.decode(encoding)?;
    }

    Ok(decoder.into())
}

pub fn decode_request(mut req: Request<Body>) -> Result<Request<Body>, EndpointError> {
    if !req.headers().contains_key(CONTENT_ENCODING) {
        return Ok(req);
    }

    if let Some(val) = req.headers_mut().remove(CONTENT_LENGTH) {
        if val == "0" {
            return Ok(req);
        }
    }

    let (mut parts, body) = req.into_parts();

    let body = {
        let encodings = parse_encodings(&parts.headers);
        decode_body(encodings, body)?
    };

    parts.headers.remove(CONTENT_ENCODING);

    Ok(Request::from_parts(parts, body))
}

pub fn decode_response(mut res: Response<Body>) -> Result<Response<Body>, EndpointError> {
    if !res.headers().contains_key(CONTENT_ENCODING) {
        return Ok(res);
    }

    if let Some(val) = res.headers_mut().remove(CONTENT_LENGTH) {
        if val == "0" {
            return Ok(res);
        }
    }

    let (mut parts, body) = res.into_parts();

    let body = {
        let encodings = parse_encodings(&parts.headers);
        decode_body(encodings, body)?
    };

    parts.headers.remove(CONTENT_ENCODING);

    Ok(Response::from_parts(parts, body))
}
