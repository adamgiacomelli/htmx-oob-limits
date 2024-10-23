# HTMX oob limits

The idea behind this prototype is to try to see how far we can take a SSE setup by utilizing HTMX out of bound updates to update a webpage in realtime.

## Stack
- [Actix web](https://actix.rs/) - A web framework in rust
- [HTMX](https://htmx.org/) - A web frontend framework that adds functionality to html, reducing the need for javascript and simplifying the frontend development process

## Running the project

```
cargo run -- -m <mode>
```

For all cli parameters run:
```
cargo run -- -h
```

## Modes
- random: This mode updates a random tile on every loop
- video: This mode updates the tiles colors according to a source video

## Notes
The [example video](https://www.pexels.com/video/woman-walking-in-high-heel-shoes-8061666/) was downloaded from [Pexels.com](https://www.pexels.com)
