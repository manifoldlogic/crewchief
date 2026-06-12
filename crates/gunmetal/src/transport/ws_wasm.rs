//! WASM WebSocket transport using web_sys::WebSocket.
//!
//! Provides client WebSocket connections for browser targets.
//! Uses `web_sys::WebSocket` with `Closure::wrap` for event handlers.
//!
//! Browser WebSockets are event-driven:
//! - `onopen` → connection established
//! - `onmessage` → data received (JSON text frames)
//! - `onclose` → connection closed
//! - `onerror` → connection error

#[derive(Debug, Clone)]
pub struct WsWasmConfig {
    pub peer_urls: Vec<String>,
    pub max_message_size: usize,
    pub binary_mode: bool,
}

impl Default for WsWasmConfig {
    fn default() -> Self {
        Self {
            peer_urls: Vec::new(),
            max_message_size: 10 * 1024 * 1024,
            binary_mode: false,
        }
    }
}

#[derive(Debug, Clone)]
pub enum WsWasmEvent {
    Open { url: String },
    Close { url: String, code: u16, reason: String },
    Error { url: String, message: String },
    Message { url: String, data: String },
}

#[cfg(target_arch = "wasm32")]
mod implementation {
    use std::cell::RefCell;
    use std::collections::HashMap;
    use std::rc::Rc;

    use wasm_bindgen::prelude::*;
    use wasm_bindgen::JsCast;
    use web_sys::{ErrorEvent, MessageEvent, WebSocket};

    use super::{WsWasmConfig, WsWasmEvent};

    /// Shared `(peer_id, message)` callback slot.
    type MessageHandler = Rc<RefCell<Option<Box<dyn Fn(String, String)>>>>;

    struct PeerConnection {
        ws: WebSocket,
        /// Kept for diagnostics/reconnect; not read on the hot path.
        #[allow(dead_code)]
        url: String,
        // Closures must be kept alive for the duration of the connection
        _on_open: Closure<dyn FnMut()>,
        _on_message: Closure<dyn FnMut(MessageEvent)>,
        _on_close: Closure<dyn FnMut(web_sys::CloseEvent)>,
        _on_error: Closure<dyn FnMut(ErrorEvent)>,
    }

    pub struct WsWasmTransport {
        config: WsWasmConfig,
        connections: Rc<RefCell<HashMap<String, PeerConnection>>>,
        events: Rc<RefCell<Vec<WsWasmEvent>>>,
        on_message: MessageHandler,
    }

    impl WsWasmTransport {
        pub fn new(config: WsWasmConfig) -> Self {
            Self {
                config,
                connections: Rc::new(RefCell::new(HashMap::new())),
                events: Rc::new(RefCell::new(Vec::new())),
                on_message: Rc::new(RefCell::new(None)),
            }
        }

        pub fn set_on_message(&self, callback: impl Fn(String, String) + 'static) {
            *self.on_message.borrow_mut() = Some(Box::new(callback));
        }

        pub fn connect(&self, url: &str) -> Result<(), String> {
            let ws = WebSocket::new(url).map_err(|e| format!("{:?}", e))?;

            let url_str = url.to_string();
            let events = self.events.clone();
            let on_msg_cb = self.on_message.clone();
            let max_size = self.config.max_message_size;

            let url_for_open = url_str.clone();
            let events_for_open = events.clone();
            let on_open = Closure::wrap(Box::new(move || {
                events_for_open.borrow_mut().push(WsWasmEvent::Open {
                    url: url_for_open.clone(),
                });
            }) as Box<dyn FnMut()>);

            let url_for_msg = url_str.clone();
            let on_message = Closure::wrap(Box::new(move |e: MessageEvent| {
                if let Ok(text) = e.data().dyn_into::<js_sys::JsString>() {
                    let s: String = text.into();
                    if s.len() <= max_size {
                        events.borrow_mut().push(WsWasmEvent::Message {
                            url: url_for_msg.clone(),
                            data: s.clone(),
                        });
                        if let Some(ref cb) = *on_msg_cb.borrow() {
                            cb(url_for_msg.clone(), s);
                        }
                    }
                }
            }) as Box<dyn FnMut(MessageEvent)>);

            let url_for_close = url_str.clone();
            let events_for_close = self.events.clone();
            let on_close = Closure::wrap(Box::new(move |e: web_sys::CloseEvent| {
                events_for_close.borrow_mut().push(WsWasmEvent::Close {
                    url: url_for_close.clone(),
                    code: e.code(),
                    reason: e.reason(),
                });
            }) as Box<dyn FnMut(web_sys::CloseEvent)>);

            let url_for_error = url_str.clone();
            let events_for_error = self.events.clone();
            let on_error = Closure::wrap(Box::new(move |e: ErrorEvent| {
                events_for_error.borrow_mut().push(WsWasmEvent::Error {
                    url: url_for_error.clone(),
                    message: e.message(),
                });
            }) as Box<dyn FnMut(ErrorEvent)>);

            ws.set_onopen(Some(on_open.as_ref().unchecked_ref()));
            ws.set_onmessage(Some(on_message.as_ref().unchecked_ref()));
            ws.set_onclose(Some(on_close.as_ref().unchecked_ref()));
            ws.set_onerror(Some(on_error.as_ref().unchecked_ref()));

            let conn = PeerConnection {
                ws,
                url: url_str.clone(),
                _on_open: on_open,
                _on_message: on_message,
                _on_close: on_close,
                _on_error: on_error,
            };

            self.connections.borrow_mut().insert(url_str, conn);
            Ok(())
        }

        pub fn send(&self, url: &str, msg: &str) -> Result<(), String> {
            let conns = self.connections.borrow();
            let conn = conns
                .get(url)
                .ok_or_else(|| format!("Not connected to: {}", url))?;
            conn.ws
                .send_with_str(msg)
                .map_err(|e| format!("{:?}", e))
        }

        pub fn broadcast(&self, msg: &str, exclude: Option<&str>) -> Result<(), String> {
            let conns = self.connections.borrow();
            for (url, conn) in conns.iter() {
                if exclude == Some(url.as_str()) {
                    continue;
                }
                if conn.ws.ready_state() == WebSocket::OPEN {
                    let _ = conn.ws.send_with_str(msg);
                }
            }
            Ok(())
        }

        pub fn close(&self, url: &str) {
            let mut conns = self.connections.borrow_mut();
            if let Some(conn) = conns.remove(url) {
                let _ = conn.ws.close();
            }
        }

        pub fn close_all(&self) {
            let mut conns = self.connections.borrow_mut();
            for (_, conn) in conns.drain() {
                let _ = conn.ws.close();
            }
        }

        pub fn connected_urls(&self) -> Vec<String> {
            let conns = self.connections.borrow();
            conns
                .iter()
                .filter(|(_, c)| c.ws.ready_state() == WebSocket::OPEN)
                .map(|(url, _)| url.clone())
                .collect()
        }

        pub fn is_connected(&self, url: &str) -> bool {
            let conns = self.connections.borrow();
            conns
                .get(url)
                .is_some_and(|c| c.ws.ready_state() == WebSocket::OPEN)
        }

        pub fn drain_events(&self) -> Vec<WsWasmEvent> {
            self.events.borrow_mut().drain(..).collect()
        }

        pub fn connect_all(&self) -> Result<(), String> {
            for url in &self.config.peer_urls {
                self.connect(url)?;
            }
            Ok(())
        }
    }
}

#[cfg(target_arch = "wasm32")]
pub use implementation::WsWasmTransport;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config() {
        let config = WsWasmConfig::default();
        assert!(config.peer_urls.is_empty());
        assert_eq!(config.max_message_size, 10 * 1024 * 1024);
        assert!(!config.binary_mode);
    }

    #[test]
    fn ws_wasm_event_types() {
        let open = WsWasmEvent::Open {
            url: "ws://localhost".into(),
        };
        let close = WsWasmEvent::Close {
            url: "ws://localhost".into(),
            code: 1000,
            reason: "normal".into(),
        };
        let msg = WsWasmEvent::Message {
            url: "ws://localhost".into(),
            data: "{}".into(),
        };

        match open {
            WsWasmEvent::Open { url } => assert_eq!(url, "ws://localhost"),
            _ => panic!("wrong variant"),
        }
        match close {
            WsWasmEvent::Close { code, .. } => assert_eq!(code, 1000),
            _ => panic!("wrong variant"),
        }
        match msg {
            WsWasmEvent::Message { data, .. } => assert_eq!(data, "{}"),
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn config_with_peers() {
        let config = WsWasmConfig {
            peer_urls: vec!["ws://a:8080".into(), "ws://b:8080".into()],
            max_message_size: 1024,
            binary_mode: true,
        };
        assert_eq!(config.peer_urls.len(), 2);
        assert!(config.binary_mode);
    }
}
