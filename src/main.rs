mod app;
mod state;
mod config;
mod error;
mod handlers;
mod middleware;
mod models;
mod services;

use dotenv::dotenv;

#[tokio::main]
async fn main() {
    dotenv().ok();

    let app = app::create_app().await;

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000")
        .await
        .expect("Failed to bind to address");

    println!("Server running on localhost:3000");

    axum::serve(listener, app)
        .await
        .expect("Failed to start server");
}