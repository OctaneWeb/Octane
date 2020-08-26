use crate::path::PathBuf;
use crate::router::{Closure, Flow};

/*
fn serve_static(mount: PathBuf, dir: PathBuf) -> Closure {
    Box::new(move |req, res| {
        Box::pin(async {
            if let Some(x) = req.request.request_line.path.subtract(&mount) {
                if let Ok(Some(_)) = res.send_file(dir.concat_owned(x).into()) {
                    return Flow::Stop;
                }
            }
            Flow::Next
        })
    })
}
*/