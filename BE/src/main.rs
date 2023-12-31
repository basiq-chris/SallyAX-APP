use BSAPI::{request_handler::send_request, Token};
use actix_web::{Responder, HttpServer, App, HttpResponseBuilder, web::{self}};
use qstring::QString;
use reqwest::{StatusCode, Client, Method, header::ACCEPT};
use basiq_api as BSAPI;
use serde_json::Value;
use std::{sync::Mutex, str::FromStr};
use Logger;
use tokio;

#[actix_web::main]
async fn main() -> Result<(), std::io::Error> {

    tokio::spawn(async {
        let client = reqwest::Client::new();
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(178600)).await;
            Logger::print_info("Starting user purger");
            let token = get_server_token().await.token;
            
            let users: Value = client.get("https://au-api.basiq.io/users")
            .bearer_auth(token.clone())
            .header(ACCEPT, "application/json").send().await.unwrap().json().await.unwrap();
            let userz = users["data"].as_array().unwrap();
            for user in userz.iter() {
                let consent: Value = client.get(format!("https://au-api.basiq.io/users/{}/consents", user["id"].as_str().unwrap()))
                .bearer_auth(token.clone())
                .header(ACCEPT, "application/json").send().await.unwrap().json().await.unwrap();

                if consent["status"].as_str().unwrap_or_else(|| "expired") == "expired" {
                    Logger::print_warning(format!("User {}, is being purged", user["id"].as_str().unwrap()));
                    drop(consent);
                    let _ = client.delete(format!("https://au-api.basiq.io/users/{}", user["id"].as_str().unwrap()))
                    .bearer_auth(token.clone())
                    .header(ACCEPT, "application/json").send();
                    Logger::print_info(format!("User {}, as been deleted", user["id"].as_str().unwrap()));
                }
            }
            drop(token);
            drop(users);
        }
    });

    let token = actix_web::web::Data::new(ServerToken {
        token: Mutex::new(get_server_token().await)
    });
    Logger::print_debug("Server created!");
    HttpServer::new(move || {
        App::new()
        .service(get_client_token)
        .service(create_user)
        .service(create_auth_link)
        .service(get_job)
        .service(job_poll)
        .app_data(token.clone())
    })
    .bind(("127.0.0.1", 8642))?
    .run()
    .await
}

struct ServerToken {
    token: Mutex<BSAPI::Token>
}

#[actix_web::get("/token/{user_id}")]
async fn get_client_token(request_body: web::Path<String>, server_token: web::Data<ServerToken>) -> impl Responder {
    Logger::print_info("GET request made to /token");
    let user_id = request_body.into_inner();
    Logger::print_debug("userID used: ".to_owned() + user_id.as_str());
    let mut token = server_token.token.lock().unwrap();
    if token.has_expired() {
        Logger::print_info("Token Expired");
        *token = get_server_token().await;
    }
    let tkn = token.clone();
    drop(token);
    HttpResponseBuilder::new(StatusCode::CREATED)
    .append_header(("Access-Control-Allow-Origin", "*"))
    .append_header(("Access-Control-Allow-Methods", "GET,POST,DELETE"))
    .append_header(("Access-Control-Allow-Headers", "Origin, X-Requested-With, Content-Type, Accept"))
    .body(BSAPI::request_handler::send_request(reqwest::Client::new(), BSAPI::RequestType::Token(BSAPI::KeyType::CLIENT_ACCESS), reqwest::Method::POST, Some(tkn), Some(user_id)).await.stringify())
}

#[actix_web::post("/createuser")]
async fn create_user(response_body: String, server_token: actix_web::web::Data<ServerToken>) -> impl Responder {
    Logger::print_info("POST request made to /createuser");
    let query = QString::from(response_body.as_str());
    Logger::print_debug(query.clone());
    Logger::print_debug("Checking Token health".to_string());
    let mut token = server_token.token.lock().unwrap();
    if token.has_expired() {
        Logger::print_info("Token Expired");
        *token = get_server_token().await;
    }
    let tkn = token.clone();
    drop(token);
    HttpResponseBuilder::new(StatusCode::CREATED)
    .append_header(("Access-Control-Allow-Origin", "*"))
    .append_header(("Access-Control-Allow-Methods", "GET,POST,DELETE"))
    .append_header(("Access-Control-Allow-Headers", "Origin, X-Requested-With, Content-Type, Accept"))
    .body(BSAPI::request_handler::send_request(Client::new(), BSAPI::RequestType::Users(vec![query.get("email").unwrap_or_else(|| "").to_string(), query.get("mobile").unwrap_or_else(|| "").to_string(), query.get("firstName").unwrap_or_else(|| "").to_string(), query.get("middleName").unwrap_or_else(|| "").to_string(), query.get("lastName").unwrap_or_else(|| "").to_string()]), reqwest::Method::POST, Some(tkn), None).await.stringify())
}

#[actix_web::post("/createauthlink")]
async fn create_auth_link(response_body: String, server_token: actix_web::web::Data<ServerToken>) -> impl Responder {
    let query = QString::from(response_body.as_str());
    Logger::print_debug(query.clone());
    Logger::print_debug("Checking token health".to_string());
    let mut token = server_token.token.lock().unwrap();
    if token.has_expired() {
        Logger::print_info("Token Expired".to_string());
        *token = get_server_token().await;
    }
    let tkn = token.clone();
    drop(token);
    Logger::print_info("POST request made to /createauthlink");
    HttpResponseBuilder::new(StatusCode::CREATED)
    .append_header(("Access-Control-Allow-Origin", "*"))
    .append_header(("Access-Control-Allow-Methods", "GET,POST,DELETE"))
    .append_header(("Access-Control-Allow-Headers", "Origin, X-Requested-With, Content-Type, Accept"))
    .body(BSAPI::request_handler::send_request(Client::new(), BSAPI::RequestType::AuthLink(query.get("userID").map(String::from).unwrap()), Method::POST, Some(tkn), None).await.stringify())
}

#[actix_web::get("/getjob/{job_id}")]
async fn get_job(url_params: web::Path<String>, server_token: actix_web::web::Data<ServerToken>) -> impl Responder {
    let query = url_params.into_inner();
    Logger::print_debug(query.clone());
    Logger::print_debug("Checking token health".to_string());
    let mut token = server_token.token.lock().unwrap();
    if token.has_expired() {
        Logger::print_info("Token expired");
        *token = get_server_token().await;
    }
    let tkn = token.clone();
    drop(token);
    Logger::print_info("GET request made to /getjob");
    HttpResponseBuilder::new(StatusCode::OK)
    .append_header(("Access-Control-Allow-Origin", "*"))
    .append_header(("Access-Control-Allow-Methods", "GET,POST,DELETE"))
    .append_header(("Access-Control-Allow-Headers", "Origin, X-Requested-With, Content-Type, Accept"))
    .body(BSAPI::request_handler::send_request(Client::new(), BSAPI::RequestType::Jobs(query), Method::GET, Some(tkn), None).await.stringify())
}

async fn get_server_token() -> Token {
    let req = send_request(reqwest::Client::new(), BSAPI::RequestType::Token(BSAPI::KeyType::SERVER_ACCESS), reqwest::Method::POST, None, None).await;
    Token::new(req.res.data)
}

#[actix_web::get("/job/{job_id}/poll")]
async fn job_poll(job_query: web::Path<String>, server_token: actix_web::web::Data<ServerToken>) -> impl Responder {
    let job_id = job_query.into_inner();
    Logger::print_debug("Polling job, ".to_owned() + job_id.as_str());
    Logger::print_debug("Checking token health".to_string());
    let mut token = server_token.token.lock().unwrap();
    if token.has_expired() {
        Logger::print_info("Token expired");
        *token = get_server_token().await;
    }
    let tkn = token.clone();
    drop(token);
    let poll_info = Value::from_str(reqwest::Client::new().get(&("https://au-api.basiq.io/jobs/".to_owned() + job_id.as_str()))
    .bearer_auth(tkn.token).send().await.unwrap().text().await.unwrap().as_str()).unwrap();
#[allow(non_snake_case)]
let mut hasFailed = false;
#[allow(non_snake_case)]
    let mut isSuccessful = false;

    if let Some(steps) = poll_info["steps"].as_array() {
        for step in steps {
            if step["status"].as_str().unwrap() == "success" {
                isSuccessful = true;
            } else if step["status"].as_str().unwrap() == "failed" {
                hasFailed = true;
            }
        }
    }
    

    if hasFailed {
        return HttpResponseBuilder::new(StatusCode::FAILED_DEPENDENCY)
            .append_header(("Access-Control-Allow-Origin", "*"))
            .append_header(("Access-Control-Allow-Methods", "GET,POST,DELETE"))
            .append_header(("Access-Control-Allow-Headers", "Origin, X-Requested-With, Content-Type, Accept"))
            .finish();
    } else if isSuccessful {
        return HttpResponseBuilder::new(StatusCode::OK)
            .append_header(("Access-Control-Allow-Origin", "*"))
            .append_header(("Access-Control-Allow-Methods", "GET,POST,DELETE"))
            .append_header(("Access-Control-Allow-Headers", "Origin, X-Requested-With, Content-Type, Accept"))
            .finish();
    } else {
        return HttpResponseBuilder::new(StatusCode::PROCESSING)
            .append_header(("Access-Control-Allow-Origin", "*"))
            .append_header(("Access-Control-Allow-Methods", "GET,POST,DELETE"))
            .append_header(("Access-Control-Allow-Headers", "Origin, X-Requested-With, Content-Type, Accept"))
            .finish();
    }
}

/* pub async fn get_all_users(token: String) -> Vec<String> {
    let req = reqwest::Client::get("https://au-api.basiq.io/users")
    .header(ACCEPT, "application/json")
    .bearer_auth(token)
    .send().await.unwrap();
} */


