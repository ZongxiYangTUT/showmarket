use actix_web::{App, HttpServer, Responder, get, web};

#[get("hello/{name}")]
async fn greet(name: web::Path<String>) -> impl Responder {
    format!("hello {name}")
}
#[actix_web::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| App::new().service(greet))
        .bind(("127.0.0.1", 8080))?
        .run()
        .await
}
