use octane::Octane;
use std::future::Future;
use tokio::runtime::{Builder, Runtime};

pub const PORT: u16 = 8080;

pub fn run<T, F>(octane: Octane, exec: T)
where
    T: Fn() -> F,
    F: Future,
{
    let mut builder = Builder::new();
    builder
        .basic_scheduler()
        .enable_io()
        .thread_name("octane-test");

    let mut runtime = builder.build().expect("Unable to build tokio runtime");

    runtime.block_on(async {
        let inner_runtime = Runtime::new().unwrap();
        inner_runtime.spawn(async {
            octane.listen(PORT, || {}).await.unwrap();
        });
        exec().await;
        inner_runtime.shutdown_background();
    });
    runtime.shutdown_background();
}

#[macro_export]
macro_rules! path {
    ( $url : expr ) => {{
        use common::PORT;
        format!("{}/{}", format!("http://0.0.0.0:{}", PORT), $url)
    }};
}

pub async fn client_request(url: &str) -> String {
    reqwest::get(url).await.unwrap().text().await.unwrap()
}
