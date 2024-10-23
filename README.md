# HTMX oob limits

The idea behind this prototype is to try to see how far we can take a SSE setup by utilizing HTMX out of bound updates to update a webpage in realtime.

## Stack
- [Actix web](https://actix.rs/) - A web framework in rust
- [HTMX](https://htmx.org/) - A web frontend framework that adds functionality to html, reducing the need for javascript and simplifying the frontend development process

## Running the project

```
cargo run <mode>
```

## Modes
- random: This mode updates a random tile on every loop
- video: This mode updates the tiles colors according to a source video
