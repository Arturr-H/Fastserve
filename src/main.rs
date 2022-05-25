mod app;
mod api;
use crate::app::server::{ ServerOptions, RouteRoot as RR, RouteValue as RV };
use crate::api::utils::{ parse_headers, HeaderReturn };
use crate::app::server;

fn main() {

    /*- The route-structure -*/
    let routes:Vec<RR> = vec![
        RR::Endpoint("",                 RV::File("index.html")),

        RR::Stack("/", vec![
            RR::Endpoint("hej",          RV::Function(api::functions::get_all_users)),
            RR::Endpoint("hejs",          RV::Function(api::functions::insert_user)),
            RR::Endpoint("function",     RV::Function(api::functions::test_fn)),
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
        url         : "127.0.0.1",
        port        : 8081,
        numthreads  : 10,
        static_files: "./static",
        routes      : routes.clone(),
        custom404   : Some("404.html"),
        log_status  : true,
        on_connect  : Some(on_connect)
    });
}

fn on_connect(request:&String) {
    println!("{:#?}", parse_headers(&request, HeaderReturn::All));
}