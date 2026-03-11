use axum::{
    body::Body,
    http::{header, StatusCode, Uri},
    response::{IntoResponse, Response},
};
use include_dir::{include_dir, Dir};

static UI_DIR: Dir = include_dir!("$CARGO_MANIFEST_DIR/ui/dist");

pub async fn serve_ui(uri: Uri) -> impl IntoResponse {
    let path = uri.path().trim_start_matches('/');

    // strip the _/ prefix that PocketBase uses for the admin UI
    let path = path.strip_prefix("_/").unwrap_or(path);
    let path = path.trim_start_matches('/');

    serve_file(path).await
}

async fn serve_file(path: &str) -> Response {
    // try exact path first
    if let Some(file) = UI_DIR.get_file(path) {
        return file_response(file);
    }

    // SPA fallback — serve index.html for any unmatched path
    if let Some(file) = UI_DIR.get_file("index.html") {
        return file_response(file);
    }

    StatusCode::NOT_FOUND.into_response()
}

fn file_response(file: &include_dir::File) -> Response {
    let mime = mime_type(file.path().to_str().unwrap_or(""));
    let bytes = file.contents().to_vec(); // copy to owned Vec<u8>
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, mime)
        .body(Body::from(bytes))
        .unwrap()
}

fn mime_type(path: &str) -> &'static str {
    if path.ends_with(".html") { "text/html; charset=utf-8" }
    else if path.ends_with(".js")   { "application/javascript" }
    else if path.ends_with(".css")  { "text/css" }
    else if path.ends_with(".svg")  { "image/svg+xml" }
    else if path.ends_with(".png")  { "image/png" }
    else if path.ends_with(".ico")  { "image/x-icon" }
    else if path.ends_with(".json") { "application/json" }
    else if path.ends_with(".woff2") { "font/woff2" }
    else if path.ends_with(".woff") { "font/woff" }
    else { "application/octet-stream" }
}