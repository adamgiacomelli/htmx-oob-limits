use actix_files as fs;
use actix_web::{get, middleware::Logger, web, App, HttpResponse, HttpServer, Responder};
use broadcast::Broadcaster;
use clap::Parser;
use hex_color::{Display, HexColor};
use maud::{html, DOCTYPE};
use rand::Rng;
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};
use tokio::time::interval;
use utils::rgb_to_rounded_hex_color_string;
use video_processor::{extract_and_process_frames, generate_grid, Tile};

mod broadcast;
mod utils;
mod video_processor;

struct AppState {
    app_name: String,
    grid: Vec<Vec<Tile>>,
    frame_data: Vec<Vec<Vec<[u8; 3]>>>,
    broadcaster: Arc<Broadcaster>,
}

#[derive(Parser, Clone)]
struct AppConfig {
    mode: String,
    #[clap(default_value = "8080")]
    port: u16,
    #[clap(default_value = "127.0.0.1")]
    bind_ip: String,
    #[clap(default_value = "15")]
    grid_size: usize,
    #[clap(default_value = "10")]
    size_rect: i32,
    #[clap(default_value = "example.mp4")]
    video_path: String,
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let args = AppConfig::parse();

    let mode = args.mode;
    let video_path = args.video_path;
    let grid_size = args.grid_size;
    let size_rect = args.size_rect;

    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));

    let frame_data = extract_and_process_frames(&video_path, grid_size)?;

    let state = Arc::new(AppState {
        app_name: String::from("HTMX grid oob"),
        grid: generate_grid(grid_size),
        frame_data,
        broadcaster: Broadcaster::create(),
    });

    let state_clone = Arc::clone(&state);

    let is_running = Arc::new(AtomicBool::new(false));
    actix_rt::spawn(async move {
        if is_running.load(Ordering::SeqCst) {
            return;
        }

        is_running.store(true, Ordering::SeqCst);

        let mut rng_gen = rand::thread_rng();
        let mut interval = interval(Duration::from_millis(60));
        let mut frame = 0;
        let mut last_frame: Vec<Vec<[u8; 3]>> = Vec::new();
        loop {
            interval.tick().await;

            if mode == "random" {
                let random_rgb: HexColor = rand::random();
                let html_color_str = Display::new(random_rgb);

                let id_row = rng_gen.gen_range(0..grid_size);
                let id_col = rng_gen.gen_range(0..grid_size);
                let id = format!("{id_row}_{id_col}");
                let body = html! {
                    #{"t"(id)} hx-swap-oob="true" style={"background:"(html_color_str)} { (id) }
                };

                state_clone.broadcaster.broadcast(&body.into_string()).await;
            } else if mode == "video" {
                for (id_row, r) in state_clone.frame_data[frame].iter().enumerate() {
                    let mut frame_update: String = Default::default();
                    for (id_col, c) in r.iter().enumerate() {
                        if !last_frame.is_empty() {
                            let html_color_str = rgb_to_rounded_hex_color_string(*c);
                            if html_color_str
                                != rgb_to_rounded_hex_color_string(last_frame[id_row][id_col])
                            {
                                let id = format!("{id_row}_{id_col}");

                                let body = html! {
                                    #{"t"(id)} style={"top:"(id_row*size_rect as usize)"px; left:"(id_col*size_rect as usize)"px;background-color:"(html_color_str)";"} hx-swap-oob="true"  {}
                                };
                                frame_update += &body.into_string();
                            }
                        }
                    }
                    // Broadcast the HTML to update the front end
                    state_clone.broadcaster.broadcast(&frame_update).await;
                }

                last_frame = state_clone.frame_data[frame].clone();
                frame += 1;
                if frame >= state_clone.frame_data.len() {
                    frame = 0;
                }
            }

            println!("Iteration done.");
            is_running.store(false, Ordering::SeqCst);
        }
    });

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
async fn index(state: web::Data<AppState>) -> impl Responder {
    let size_rect = AppConfig::parse().size_rect;

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
                    @for row in &state.grid {
                        @for col in row {
                           #{"t"(col.id)} style={"left:"(col.x*size_rect)"px; top:"(col.y*size_rect)"px;"}  { }
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
async fn event_stream(state: web::Data<AppState>) -> impl Responder {
    state.broadcaster.new_client().await
}

#[get("/data")]
async fn data() -> impl Responder {
    let body = html! {
        span #"tile_2_4" .tile hx-swap-oob="true" style={"background:red"} { "2_4" }
    };

    HttpResponse::Ok().body(body.into_string())
}
