//! Prometheus metrics endpoint and HTTP server.
//!
//! This module provides an HTTP server that exposes Prometheus metrics
//! for scraping by monitoring systems.

use crate::metrics::search_metrics::get_registry;
use prometheus::{Encoder, TextEncoder};
use std::net::SocketAddr;
use tokio::net::TcpListener;
use tracing::{error, info};

/// Initialize and start the Prometheus metrics HTTP server.
///
/// This starts a simple HTTP server on the specified address that serves
/// Prometheus metrics at the `/metrics` endpoint.
///
/// # Arguments
/// * `addr` - Socket address to bind to (e.g., "0.0.0.0:9090")
///
/// # Example
/// ```no_run
/// use crewchief_maproom::metrics::init_metrics_server;
///
/// #[tokio::main]
/// async fn main() {
///     tokio::spawn(async {
///         if let Err(e) = init_metrics_server("0.0.0.0:9090").await {
///             eprintln!("Metrics server error: {}", e);
///         }
///     });
///
///     // ... rest of application ...
/// }
/// ```
pub async fn init_metrics_server(addr: &str) -> Result<(), Box<dyn std::error::Error>> {
    let addr: SocketAddr = addr.parse()?;
    let listener = TcpListener::bind(addr).await?;

    info!("Prometheus metrics server listening on {}", addr);
    info!("Metrics available at http://{}/metrics", addr);

    loop {
        match listener.accept().await {
            Ok((mut stream, peer_addr)) => {
                tokio::spawn(async move {
                    if let Err(e) = handle_connection(&mut stream, peer_addr).await {
                        error!("Error handling connection from {}: {}", peer_addr, e);
                    }
                });
            }
            Err(e) => {
                error!("Error accepting connection: {}", e);
            }
        }
    }
}

/// Handle an incoming HTTP connection.
async fn handle_connection(
    stream: &mut tokio::net::TcpStream,
    peer_addr: SocketAddr,
) -> Result<(), Box<dyn std::error::Error>> {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let mut buffer = [0; 1024];
    let n = stream.read(&mut buffer).await?;
    let request = String::from_utf8_lossy(&buffer[..n]);

    // Parse the request line
    let first_line = request.lines().next().unwrap_or("");
    let parts: Vec<&str> = first_line.split_whitespace().collect();

    if parts.len() >= 2 {
        let method = parts[0];
        let path = parts[1];

        if method == "GET" && path == "/metrics" {
            info!("Serving metrics to {}", peer_addr);
            let response = metrics_handler();
            stream.write_all(response.as_bytes()).await?;
        } else if method == "GET" && path == "/" {
            // Root path - show help message
            let help_response = format!(
                "HTTP/1.1 200 OK\r\n\
                 Content-Type: text/html\r\n\
                 \r\n\
                 <html>\
                 <head><title>Maproom Metrics</title></head>\
                 <body>\
                 <h1>Maproom Search Metrics</h1>\
                 <p>Metrics are available at <a href=\"/metrics\">/metrics</a></p>\
                 <h2>Available Metrics</h2>\
                 <ul>\
                 <li><code>maproom_search_query_latency_seconds</code> - Query latency histogram</li>\
                 <li><code>maproom_search_fusion_time_seconds</code> - Fusion time histogram</li>\
                 <li><code>maproom_search_cache_hit_rate</code> - Cache hit rate gauge</li>\
                 <li><code>maproom_search_result_count</code> - Result count histogram</li>\
                 <li><code>maproom_search_errors_total</code> - Error counter</li>\
                 <li><code>maproom_search_queries_total</code> - Query counter</li>\
                 </ul>\
                 </body>\
                 </html>"
            );
            stream.write_all(help_response.as_bytes()).await?;
        } else {
            // 404 for other paths
            let not_found = "HTTP/1.1 404 NOT FOUND\r\n\r\n";
            stream.write_all(not_found.as_bytes()).await?;
        }
    } else {
        // Invalid request
        let bad_request = "HTTP/1.1 400 BAD REQUEST\r\n\r\n";
        stream.write_all(bad_request.as_bytes()).await?;
    }

    Ok(())
}

/// Generate the Prometheus metrics response.
///
/// This encodes all registered metrics in Prometheus text format.
///
/// # Returns
/// HTTP response string with metrics in Prometheus format
pub fn metrics_handler() -> String {
    let encoder = TextEncoder::new();
    let registry = get_registry();

    let metric_families = registry.gather();
    let mut buffer = Vec::new();

    match encoder.encode(&metric_families, &mut buffer) {
        Ok(_) => {
            let body = String::from_utf8_lossy(&buffer);
            format!(
                "HTTP/1.1 200 OK\r\n\
                 Content-Type: {}; charset=utf-8\r\n\
                 Content-Length: {}\r\n\
                 \r\n\
                 {}",
                encoder.format_type(),
                body.len(),
                body
            )
        }
        Err(e) => {
            error!("Failed to encode metrics: {}", e);
            format!(
                "HTTP/1.1 500 INTERNAL SERVER ERROR\r\n\
                 Content-Type: text/plain\r\n\
                 \r\n\
                 Failed to encode metrics: {}",
                e
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_handler_format() {
        let response = metrics_handler();

        // Should start with HTTP/1.1 200 OK
        assert!(response.starts_with("HTTP/1.1 200 OK"));

        // Should contain Content-Type header
        assert!(response.contains("Content-Type:"));

        // Should contain metrics in the body (after blank line)
        assert!(response.contains("\r\n\r\n"));
    }

    #[test]
    fn test_metrics_handler_contains_metrics() {
        // Record some metrics first with unique labels to avoid conflicts
        let metrics = crate::metrics::get_metrics();
        metrics.record_query_latency(0.025, "prometheus_test", true);
        metrics.increment_queries("prometheus_test", true);

        let response = metrics_handler();

        // Should contain our metric names or Prometheus metadata
        assert!(
            response.contains("maproom_search")
                || response.contains("# HELP")
                || response.contains("# TYPE")
        );
    }

    #[tokio::test]
    async fn test_socket_addr_parsing() {
        // Test valid addresses
        let addr: Result<SocketAddr, _> = "127.0.0.1:9090".parse();
        assert!(addr.is_ok());

        let addr: Result<SocketAddr, _> = "0.0.0.0:8080".parse();
        assert!(addr.is_ok());

        // Test invalid address
        let addr: Result<SocketAddr, _> = "invalid".parse();
        assert!(addr.is_err());
    }
}
