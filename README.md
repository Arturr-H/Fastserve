# Rust server handler
Simple and clean, here's how you can set it up! 👇

```rust
mod app;
mod api;
use crate::app::server::{ ServerOptions, RouteRoot as RR, RouteValue as RV };
use crate::api::utils::{ parse_headers, HeaderReturn };
use crate::app::server;

fn main() {

    /*- The route-structure -*/
    let routes:Vec<RR> = vec![
        RR::Endpoint("",                         RV::File("index.html")),

        RR::Stack("/", vec![
            RR::Endpoint("hejs",                 RV::Function(some_func)),
            RR::Endpoint("function",             RV::Function(other_func)),
        ]),

        RR::Stack("/api", vec![
            RR::Stack("/v2", vec![
                RR::Endpoint("some_endpoint",    RV::File("someFile.html")),
            ]),
        ]),
    ];

    /*- Start the server -*/
    server::start(ServerOptions {
        url         : "127.0.0.1",      // Use 0.0.0.0 if using ex Docker
        port        : 8081,             // The http-port you want to use
        numthreads  : 10,               // Amount of clients that can join concurrently
        routes      : routes.clone(),   // The route-structure
        custom404   : Some("404.html"), // Default is '404.html'
        log_status  : true,             // Will log things, like when the server starts
        on_connect  : Some(on_connect)  // Do something when a user is connected
        statics   : Statics {
            dir      : "./static",       // The directory where you put your static files
            custom404: Some("404.html"), // Defaults to ''404.html' if None
            serve    : true, // Serve all files in static dir even if not provided in routes
        },
    });
}

fn on_connect(request:&String) {
    println!("{:#?}", parse_headers(&request, HeaderReturn::All));
}
```