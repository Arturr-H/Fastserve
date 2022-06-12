mod api;
mod app;
mod global;
use crate::app::server::{ ServerOptions, Statics, RouteRoot as RR, RouteValue as RV };
use crate::app::server;
use crate::global::{ PORT, URL };

fn main() {

    /*- The route-structure -*/
    let routes:Vec<RR> = vec![
        RR::Endpoint("",                 RV::File("index.html")),

        RR::Stack("/", vec![
            RR::Endpoint("website/:url", RV::Function(api::functions::google_test)),
            RR::Endpoint("start/:url/:shit/end", RV::Function(api::functions::param_test)),
            RR::Endpoint("test",         RV::Function(api::functions::test_fn)),
            
            RR::Endpoint("artur/:local",         RV::Function(api::functions::get_artur_dir)),
            RR::Endpoint("terminal/:command",         RV::Function(api::functions::terminal)),
        ]),

        RR::Stack("/api", vec![
            RR::Stack("/shit", vec![
                RR::Endpoint("hej",      RV::File("index.html")),
                RR::Endpoint("hej2",     RV::File("index.html"))
            ]),
        ]),
    ];

    /*- Start the server -*/
    server::start(ServerOptions {
        routes    : routes.clone(),
        url       : URL,
        port      : PORT,
        numthreads: 10,
        statics   : Statics {
            dir      : "./static",
            custom404: Some("404.html"),
            serve    : true,
        },
        log_status: true,
        on_connect: None,
    });
}