use axum::{
    extract::State,
    http::{header::CONTENT_TYPE, HeaderValue},
    response::{IntoResponse, Response},
    routing::get,
    Router,
};

mod model;
use crate::model::*;

#[derive(Clone, Debug)]
pub struct AppState {
    pub producer: winit::event_loop::EventLoopProxy<UserEvent>,
}

async fn handler(State(state): State<AppState>) -> Response {
    let (tx, rx) = tokio::sync::oneshot::channel();

    state
        .producer
        .send_event(UserEvent::RenderingRequest(tx))
        .unwrap();

    (
        [(CONTENT_TYPE, HeaderValue::from_str("image/png").unwrap())],
        rx.await.unwrap(),
    )
        .into_response()
}

pub async fn run(listen: &str, proxy: winit::event_loop::EventLoopProxy<UserEvent>) {
    let shutdowner = model::Shutdowner::new(proxy.clone());
    let app = Router::new()
        .route("/", get(handler))
        .with_state(AppState { producer: proxy });

    let listener = tokio::net::TcpListener::bind(listen).await.unwrap();
    axum::serve(listener, app).await.unwrap();

    std::mem::drop(shutdowner);
}
