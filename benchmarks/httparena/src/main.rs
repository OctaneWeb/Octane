//! Octane's entry for HttpArena: every endpoint goes through octane's router,
//! request and response API, on the multi-threaded tokio runtime.

use octane::prelude::*;
use octane::StatusCode;
use serde::{Deserialize, Serialize};
use octane::request::Query;
use std::error::Error;
use std::time::Duration;

#[derive(Deserialize)]
struct Rating {
    score: i64,
    count: i64,
}

#[derive(Deserialize)]
struct DatasetItem {
    id: i64,
    name: String,
    category: String,
    price: i64,
    quantity: i64,
    active: bool,
    tags: Vec<String>,
    rating: Rating,
}

#[derive(Serialize)]
struct RatingOut {
    score: i64,
    count: i64,
}

// Field order is the wire order: id..rating, then the computed total.
#[derive(Serialize)]
struct ProcessedItem<'a> {
    id: i64,
    name: &'a str,
    category: &'a str,
    price: i64,
    quantity: i64,
    active: bool,
    tags: &'a [String],
    rating: RatingOut,
    total: i64,
}

#[derive(Serialize)]
struct JsonResponse<'a> {
    items: Vec<ProcessedItem<'a>>,
    count: usize,
}

fn load_dataset() -> Vec<DatasetItem> {
    let path = std::env::var("DATASET_PATH").unwrap_or_else(|_| "/data/dataset.json".to_string());
    match std::fs::read_to_string(path) {
        Ok(data) => serde_json::from_str(&data).unwrap_or_default(),
        Err(_) => Vec::new(),
    }
}

// A query parameter as an integer, 0 when it is missing or isn't one
fn int(query: &Query, name: &str) -> i64 {
    query
        .get(name)
        .and_then(|value| value.parse().ok())
        .unwrap_or(0)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Leaked once at startup so handlers borrow the items instead of cloning
    // every string into each response.
    let dataset: &'static [DatasetItem] = Box::leak(load_dataset().into_boxed_slice());

    let mut app = Octane::new();
    // The async profile holds 32,000 connections open at once, octane serves
    // 1,024 at a time by default.
    app.settings.max_connections = 65_536;

    app.get(
        "/pipeline",
        route_stop!(|req, res| {
            res.with_type("text/plain").send("ok");
        }),
    )?;

    app.get(
        "/baseline11",
        route_stop!(|req, res| {
            let query = req.get_query();
            let sum = int(&query, "a") + int(&query, "b");
            res.with_type("text/plain").send_owned(sum.to_string());
        }),
    )?;

    // The body arrives whole, whether it was sent with a Content-Length or
    // chunked.
    app.post(
        "/baseline11",
        route_stop!(|req, res| {
            let query = req.get_query();
            let body = std::str::from_utf8(req.body)
                .ok()
                .and_then(|body| body.trim().parse::<i64>().ok())
                .unwrap_or(0);
            let sum = int(&query, "a") + int(&query, "b") + body;
            res.with_type("text/plain").send_owned(sum.to_string());
        }),
    )?;

    // An async handler: the request waits on a tokio timer, its worker thread
    // goes on serving other connections.
    app.get(
        "/delay/:ms",
        route_async_stop!(|req, res| {
            match req.vars.get("ms").and_then(|ms| ms.parse::<u64>().ok()) {
                Some(ms) => {
                    tokio::time::sleep(Duration::from_millis(ms)).await;
                    res.with_type("text/plain").send_owned(ms.to_string());
                }
                None => {
                    res.status(StatusCode::BadRequest)
                        .with_type("text/plain")
                        .send("bad delay");
                }
            }
        }),
    )?;

    // The first `count` items with their totals, serialized per request.
    app.get(
        "/json/:count",
        route_stop!(|req, res| {
            match req.vars.get("count").and_then(|count| count.parse::<usize>().ok()) {
                Some(count) => {
                    let m = req
                        .get_query()
                        .get("m")
                        .and_then(|m| m.parse::<i64>().ok())
                        .unwrap_or(1);
                    let count = count.min(dataset.len());
                    let items = dataset[..count]
                        .iter()
                        .map(|item| ProcessedItem {
                            id: item.id,
                            name: &item.name,
                            category: &item.category,
                            price: item.price,
                            quantity: item.quantity,
                            active: item.active,
                            tags: &item.tags,
                            rating: RatingOut {
                                score: item.rating.score,
                                count: item.rating.count,
                            },
                            total: item.price * item.quantity * m,
                        })
                        .collect();
                    if res.json(JsonResponse { items, count }).is_err() {
                        res.status(StatusCode::InternalServerError).send("");
                    }
                }
                None => {
                    res.status(StatusCode::BadRequest)
                        .with_type("text/plain")
                        .send("bad count");
                }
            }
        }),
    )?;

    // The bytes that arrived, unchanged.
    app.post(
        "/echo",
        route_stop!(|req, res| {
            res.with_type("application/octet-stream").send(req.body);
        }),
    )?;

    // json-tls and 8gbit: the same routes over HTTP/1.1 + TLS on 8081 with
    // the certificate the harness mounts. Without it octane logs a warning
    // and serves plaintext only.
    let cert = std::env::var("TLS_CERT").unwrap_or_else(|_| "/certs/server.crt".to_string());
    let key = std::env::var("TLS_KEY").unwrap_or_else(|_| "/certs/server.key".to_string());
    app.ssl(8081).cert(&cert).key(&key);

    app.listen(8080, || println!("octane listening on :8080, and on :8081 over TLS"))
        .await
}
