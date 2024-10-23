use actix_files as fs;
use actix_web::{get, middleware::Logger, web, App, HttpResponse, HttpServer, Responder};
use broadcast::Broadcaster;
use clap::Parser;
use maud::{html, DOCTYPE};
use std::sync::Arc;
use stream_worker::start_sse_worker;
use video_processor::extract_and_process_frames;

mod broadcast;
mod stream_worker;
mod utils;
mod video_processor;

pub struct ServersideState {
    app_name: String,
    frame_data: Option<Vec<Vec<Vec<[u8; 3]>>>>,
    broadcaster: Arc<Broadcaster>,
}

#[derive(Parser, Clone)]
struct CliConfiguration {
    #[arg(short)]
    mode: String,
    #[arg(short)]
    #[clap(default_value = "8080")]
    port: u16,
    #[arg(short)]
    #[clap(default_value = "127.0.0.1")]
    bind_ip: String,
    #[arg(short)]
    #[clap(default_value = "15")]
    grid_size: usize,
    #[arg(short)]
    #[clap(default_value = "10")]
    size_rect: i32,
    #[arg(short)]
    #[clap(default_value = "./example.mp4")]
    video_path: String,
    #[arg(short)]
    #[clap(default_value = "30")]
    update_frequency: i32,
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let args = CliConfiguration::parse();

    let mode = args.mode;
    let video_path = args.video_path;
    let grid_size = args.grid_size;
    let size_rect = args.size_rect;
    let update_frequency = args.update_frequency;

    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));

    let mut frame_data = None;
    if mode == "video" {
        frame_data = Some(extract_and_process_frames(&video_path, grid_size)?);
    }

    let state = Arc::new(ServersideState {
        app_name: String::from("HTMX grid oob"),
        frame_data,
        broadcaster: Broadcaster::create(),
    });

    let state_clone = Arc::clone(&state);

    start_sse_worker(mode, grid_size, size_rect, update_frequency, state_clone);

    log::info!("starting HTTP server at {}:{}", args.bind_ip, args.port);

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::from(Arc::clone(&state)))
            .service(index)
            .service(event_stream)
            .service(data)
            .service(fs::Files::new("/", "./public"))
            .wrap(Logger::default())
    })
    .bind((args.bind_ip, args.port))?
    .run()
    .await
}

#[get("/")]
async fn index(state: web::Data<ServersideState>) -> impl Responder {
    let size_rect = CliConfiguration::parse().size_rect;
    let grid_size = CliConfiguration::parse().grid_size as i32;

    let body = html! {
        (DOCTYPE)
        html {
            head {
                title  { (state.app_name) }
                link rel="stylesheet" href="style.css" {}
                script src="https://unpkg.com/htmx.org@2.0.3" {}
                script src="https://unpkg.com/htmx-ext-sse@2.2.2/sse.js" {}
            }
            body {
                h1 { "Htmx OOB grid" }
                div hx-get="/data" hx-trigger="load" {}
                .wrapper {
                    @for id_row in 0..grid_size {
                        @for id_col in 0..grid_size {
                           #{"t"(format!("{id_row}_{id_col}"))} style={"left:"(id_row*size_rect)"px; top:"(id_col*size_rect)"px;"}  { }
                        }
                    }
                }
                div hx-ext="sse" sse-connect="/events" sse-swap="message" {}
            }
        }
    };

    HttpResponse::Ok()
        .content_type("text/html")
        .body(body.into_string())
}

#[get("/events")]
async fn event_stream(state: web::Data<ServersideState>) -> impl Responder {
    state.broadcaster.new_client().await
}

#[get("/data")]
async fn data() -> impl Responder {
    let body = html! {
        span #"tile_2_4" .tile hx-swap-oob="true" style={"background:red"} { "2_4" }
    };

    HttpResponse::Ok().body(body.into_string())
}
