use axum::{Router, extract::State, http::StatusCode, routing::get};

#[derive(Clone)]
struct AppState {
    healthy: bool,
}

#[tokio::main]
async fn main() {
    let healthy = rand_bool();

    if healthy {
        println!("Starting in healthy mode");
    } else {
        println!("Starting in unhealthy mode");
    }

    let addr = "0.0.0.0:8080";

    let app = Router::new()
        .route("/", get(index))
        .route("/health", get(health))
        .with_state(AppState { healthy });

    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();

    axum::serve(listener, app).await.unwrap();
}

async fn index() -> &'static str {
    "Hello from the webservice!"
}

async fn health(State(state): State<AppState>) -> StatusCode {
    if state.healthy {
        StatusCode::OK
    } else {
        StatusCode::INTERNAL_SERVER_ERROR
    }
}

fn rand_bool() -> bool {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .subsec_nanos();

    nanos % 2 == 0
}
