#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SseEvent {
    pub event: Option<String>,
    pub data: String,
    pub id: Option<String>,
    pub retry: Option<u64>,
}

#[derive(Default)]
pub(crate) struct SseParser {
    buffer: String,
    event: Option<String>,
    data: Vec<String>,
    id: Option<String>,
    retry: Option<u64>,
    has_data: bool,
}

impl SseParser {
    pub(crate) fn push_bytes(&mut self, bytes: &[u8]) -> Vec<SseEvent> {
        self.buffer.push_str(&String::from_utf8_lossy(bytes));

        let mut events = Vec::new();

        while let Some(newline_index) = self.buffer.find('\n') {
            let mut line = self.buffer[..newline_index].to_string();
            self.buffer.drain(..=newline_index);

            if line.ends_with('\r') {
                line.pop();
            }

            if let Some(event) = self.push_line(&line) {
                events.push(event);
            }
        }

        events
    }

    fn push_line(&mut self, line: &str) -> Option<SseEvent> {
        if line.is_empty() {
            return self.dispatch();
        }

        if line.starts_with(':') {
            return None;
        }

        let (field, value) = match line.split_once(':') {
            Some((field, value)) => (field, value.strip_prefix(' ').unwrap_or(value)),
            None => (line, ""),
        };

        match field {
            "event" => self.event = Some(value.to_string()),
            "data" => {
                self.has_data = true;
                self.data.push(value.to_string());
            }
            "id" => {
                if !value.contains('\0') {
                    self.id = Some(value.to_string());
                }
            }
            "retry" => {
                if let Ok(retry) = value.parse::<u64>() {
                    self.retry = Some(retry);
                }
            }
            _ => {}
        }

        None
    }

    fn dispatch(&mut self) -> Option<SseEvent> {
        if !self.has_data && self.event.is_none() && self.id.is_none() && self.retry.is_none() {
            return None;
        }

        let event = SseEvent {
            event: self.event.take(),
            data: std::mem::take(&mut self.data).join("\n"),
            id: self.id.take(),
            retry: self.retry.take(),
        };
        self.has_data = false;

        Some(event)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sse_reassembles_event_split_across_chunks() {
        let mut parser = SseParser::default();

        assert!(parser.push_bytes(b"event: mes").is_empty());
        assert!(parser.push_bytes(b"sage\ndata: hel").is_empty());
        let events = parser.push_bytes(b"lo\nid: 42\n\n");

        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event.as_deref(), Some("message"));
        assert_eq!(events[0].data, "hello");
        assert_eq!(events[0].id.as_deref(), Some("42"));
        assert_eq!(events[0].retry, None);
    }

    #[test]
    fn sse_joins_multiline_data_and_parses_retry() {
        let mut parser = SseParser::default();
        let events = parser.push_bytes(b"retry: 1500\ndata: one\ndata: two\n\n");

        assert_eq!(events.len(), 1);
        assert_eq!(events[0].data, "one\ntwo");
        assert_eq!(events[0].retry, Some(1500));
    }

    #[test]
    fn sse_ignores_comment_heartbeat_without_dispatching_event() {
        let mut parser = SseParser::default();

        assert!(parser.push_bytes(b": heartbeat\n\n").is_empty());
    }
}
