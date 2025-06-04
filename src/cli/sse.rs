//! Server-Sent Events (SSE) parsing module for handling streaming responses.

use std::pin::Pin;
use std::task::{Context as TaskContext, Poll};

use anyhow::Result;
use bytes::Bytes;
use serde_json::Value;
use tokio_stream::Stream;

/// A stream that parses SSE events from a byte stream and extracts JSON data.
pub struct SseStream<S> {
    inner: S,
    buffer: String,
}

impl<S> SseStream<S>
where
    S: Stream<Item = Result<Bytes, reqwest::Error>> + Unpin,
{
    /// Creates a new SSE stream from a byte stream.
    pub fn new(stream: S) -> Self {
        Self {
            inner: stream,
            buffer: String::new(),
        }
    }

    /// Parses SSE events from the buffer and extracts JSON data.
    fn parse_sse_events(&mut self) -> Vec<Result<Value>> {
        let mut events = Vec::new();

        // Process complete lines in the buffer
        while let Some(line_end) = self.buffer.find('\n') {
            let line = self.buffer[..line_end].trim_end_matches('\r');

            // Extract data from SSE event
            if let Some(data) = line.strip_prefix("data: ") {
                match serde_json::from_str::<Value>(data) {
                    Ok(json) => events.push(Ok(json)),
                    Err(e) => events.push(Err(anyhow::anyhow!("Failed to parse JSON: {}", e))),
                }
            }
            // Ignore other SSE fields like event:, id:, retry:, or empty lines

            // Remove processed line from buffer
            self.buffer.drain(..=line_end);
        }

        events
    }
}

impl<S> Stream for SseStream<S>
where
    S: Stream<Item = Result<Bytes, reqwest::Error>> + Unpin,
{
    type Item = Result<Value>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut TaskContext<'_>) -> Poll<Option<Self::Item>> {
        // First, check if we have any complete events in the buffer
        let events = self.parse_sse_events();
        if let Some(event) = events.into_iter().next() {
            return Poll::Ready(Some(event));
        }

        // If not, try to get more data from the inner stream
        match Pin::new(&mut self.inner).poll_next(cx) {
            Poll::Ready(Some(Ok(bytes))) => {
                // Append new data to buffer
                match std::str::from_utf8(&bytes) {
                    Ok(text) => self.buffer.push_str(text),
                    Err(e) => {
                        return Poll::Ready(Some(Err(anyhow::anyhow!("Invalid UTF-8: {}", e))));
                    }
                }

                // Try parsing again with the new data
                let events = self.parse_sse_events();
                if let Some(event) = events.into_iter().next() {
                    Poll::Ready(Some(event))
                } else {
                    // Need more data to complete an event
                    cx.waker().wake_by_ref();
                    Poll::Pending
                }
            }
            Poll::Ready(Some(Err(e))) => {
                Poll::Ready(Some(Err(anyhow::anyhow!("Stream error: {}", e))))
            }
            Poll::Ready(None) => {
                // Stream ended, check if there's any remaining data in buffer
                if !self.buffer.trim().is_empty() {
                    // Try to parse any remaining complete events
                    let events = self.parse_sse_events();
                    if let Some(event) = events.into_iter().next() {
                        return Poll::Ready(Some(event));
                    }
                }
                Poll::Ready(None)
            }
            Poll::Pending => Poll::Pending,
        }
    }
}

/// Converts a reqwest byte stream into an SSE event stream.
pub fn parse_sse_stream(
    stream: impl Stream<Item = Result<Bytes, reqwest::Error>> + Unpin,
) -> impl Stream<Item = Result<Value>> {
    SseStream::new(stream)
}
