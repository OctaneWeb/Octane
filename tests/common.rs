use octane::Octane;
use std::future::Future;
use tokio::runtime::Builder;

pub const PORT: u16 = 8080;

pub fn run<T, F>(octane: Octane, exec: T)
where
    T: Fn() -> F,
    F: Future,
{
    let mut builder = Builder::new_multi_thread();
    builder.enable_all().thread_name("octane-test");

    let runtime = builder.build().expect("Unable to build tokio runtime");

    runtime.block_on(async {
        let handle = tokio::spawn(async {
            octane.listen(PORT, || {}).await.unwrap();
        });
        exec().await;
        handle.await.unwrap();
    });
}

#[macro_export]
macro_rules! path {
    ( $url : expr ) => {{
        use common::PORT;
        format!("{}/{}", format!("http://0.0.0.0:{}", PORT), $url)
    }};
}

#[allow(dead_code)]
pub async fn client_request(url: &str) -> String {
    reqwest::get(url).await.unwrap().text().await.unwrap()
}
