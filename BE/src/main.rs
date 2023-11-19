use Basiq_API;
use SXL;
use actix_web::HttpServer;


#[actix_web::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| {
        App::new()
    })
}

