mod app;
mod api;
use crate::app::server::{ ServerOptions, Statics, RouteRoot as RR, RouteValue as RV };
use crate::app::server;

fn main() {

    /*- The route-structure -*/
    let routes:Vec<RR> = vec![
        RR::Endpoint("",                 RV::File("index.html")),

        RR::Stack("/", vec![
            RR::Endpoint("website/:url", RV::Function(api::functions::google_test)),
            RR::Endpoint("hej/:url/:shit", RV::Function(api::functions::param_test)),
            RR::Endpoint("hejs",         RV::Function(api::functions::insert_user)),
            RR::Endpoint("function",     RV::Function(api::functions::get_all_users)),
            RR::Endpoint("test",         RV::Function(api::functions::test_fn)),
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
        url       : "127.0.0.1",   //127.0.0.1
        port      : 8081,
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