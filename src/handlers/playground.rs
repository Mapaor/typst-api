use axum::response::Html;

pub async fn playground_handler() -> Html<&'static str> {
    Html(include_str!("../../static/playground.html"))
}
