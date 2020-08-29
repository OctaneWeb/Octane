use crate::path::PathBuf;
use crate::route;
use crate::router::{Closure, Flow};

pub fn serve_static(mount: &'static str, dir: &'static str) -> Closure {
    route!(|req, res| {
        let mout = PathBuf::parse(mount).ok().unwrap();
        let dir = PathBuf::parse(dir).ok().unwrap();
        if let Some(x) = req.request.request_line.path.subtract(&mout) {
            if let Ok(Some(_)) = res.send_file(&dir.concat_owned(x).to_string()) {
                return Flow::Stop;
            }
        }
        Flow::Next
    })
}
