pub mod server {
    #![allow(dead_code, deprecated)]
    
    /*- Imports -*/
    use std::net::TcpListener;
    use std::net::TcpStream;
    use std::io::prelude::*;
    use webserver::ThreadHandler;
    use std::fs;
    use termcolor::{ Color };
    use crate::api::utils::log;

    /*- The options that the user has before starting the server -*/
    #[derive(Clone)]
    pub struct ServerOptions {
        pub url:&'static str,
        pub port:usize,
        pub numthreads:usize,
        pub static_files:&'static str,
        pub routes:Vec<RouteRoot>,
        pub custom404:Option<&'static str>,
        pub log_status:bool,
        pub on_connect:Option<fn(&String)>
    }
    
    /*- Ok to explain the RouteRoot enum -> -*/
    // I want a simple way of adding paths / routes to the server.
    // And currently without this method, you'll need to write ugly code, like this:
    // /path/subpath ---- /path/subpath2 ---- /path/subpath3 ---- /path/subpath4 and so on.
    // So instead I made a Stack-based data structure. And this is how it works:
    // ("/path", [
    //    "subpath",
    //    "subpath2",
    //    "subpath3",
    // ])
    // ("/otherPath", [ ... ])
    #[derive(Debug, Clone)]
    pub enum RouteRoot {
        Stack(&'static str, Vec<RouteRoot>),
        Endpoint(&'static str, RouteValue),
    }
    
    /*- Routes can either be a filepath, or a function -*/
    #[derive(Copy, Clone, Debug)]
    pub enum RouteValue {
        File(&'static str),
        Function(fn(TcpStream, String) -> ()),
        None
    }
    
    /*- Main startup -*/
    pub fn start(options:ServerOptions) {

        /*- The server will be active here -*/
        let server_url = format!("{}:{}",
            options.url, options.port.to_string(),
        );

        /*- If used in production, change unwrap to something that handles errors -*/
        let server_listener:TcpListener = TcpListener::bind(&server_url).unwrap();

        /*- Log -*/
        if options.log_status { log(Color::Rgb(255, 255, 0), format!("Server open on {}", &server_url).as_str()) };
    
        /*- Create a thread handler with 4 threads (users can join at same time) -*/
        let thread_handler = ThreadHandler::new(options.numthreads);

        /*- Start listening for connections -*/
        for stream_in in server_listener.incoming() {
            /*- Stream in will return a result, unwrap -*/
            let stream = stream_in.unwrap();
    
            /*- Get the options -*/
            let opt = options.clone();

            /*- Create a thread to handle the connection -*/
            thread_handler.exec(|| {
                handle_connect(stream, opt);
            });
        }
    }
    
    /*- Handle all server connection -*/
    pub fn handle_connect(mut stream:TcpStream, options:ServerOptions) {
        /*- Stream content buffer -*/
        let mut data_buffer:[u8;1024]=[0;1024];
    
        /*- Read the data and store it in the buffer -*/
        stream.read(&mut data_buffer).unwrap_or(0);
    
        /*- Get the request -*/
        let request = String::from_utf8_lossy(&data_buffer[..]);

        /*- On connect func -*/
        if let Some(on_connect) = options.on_connect { on_connect(&request.to_string()) };
    
        /*- Execute the path - either send a file or execute a function -*/
        exec_path(request.to_string(), &mut stream, options);
    }
    
    /*- Remove the trailing slash from a string -*/
    pub fn trail(path:&str) -> String {
        /*- If the path ends with a slash, remove it -*/
        if path.ends_with("/") {
            return path[0..path.len()-1].to_string();
        }
    
        /*- Otherwise, return the path -*/
        return path.to_string();
    }
    
    /*- A way of getting the URL route, or a return function -*/
    pub fn exec_path(request:String, stream:&mut TcpStream, options:ServerOptions) -> () {
        /*- Get the path from the request -*/
        let path = request.split("\n").nth(0).unwrap();
        let path = path.split(" ").nth(1).unwrap();
    
        /*- Iterate over all of them -*/
        let value = iterate_routes(&options.routes, path, 0u8, "", &options);

        /*- Get the users prefered 404 file -*/
        let custom_404 = options.custom404.unwrap_or("404.html");
    
        /*- See if the value is either a function or a file -*/
        match value {
            RouteValue::File(file_path) => return send_file(stream, file_path, &options.static_files),
            RouteValue::Function(func) => func(stream.try_clone().unwrap(), request),
            RouteValue::None => return send_file(stream, custom_404, &options.static_files),
        };
    }
    
    /*- Iterate over all of the routes to find the path's value -*/
    pub fn iterate_routes(routes:&Vec<RouteRoot>, input_path:&str, index:u8, path_iter:&str, options:&ServerOptions) -> RouteValue {
    
        /*- What to finnaly return -*/
        let mut return_value:RouteValue = RouteValue::None;
    
        /*- Iterate -*/
        'main: for route in routes.iter() {
    
            /*- Check wether the route is a stack, or a path -*/
            match route {
                RouteRoot::Stack(path,routes) => {
                    let possible_route = iterate_routes(&routes.clone(), input_path, index+1, (path_iter.to_string().clone() + *path).as_str(), &options);
                    
                    /*- If the route is a file, return it -*/
                    match possible_route {
                        RouteValue::File(file_path) => return_value =  RouteValue::File(file_path),
                        RouteValue::Function(func) => return_value =  RouteValue::Function(func),
                        RouteValue::None => (),
                    };
                },
                RouteRoot::Endpoint(enpoint_name, path) => {
    
                    /*- The full path to differentiate -*/
                    let full_path:String;
    
                    /*- Fixing som e problems with trailing slashes to avoid double slashes ("//") -*/
                    if !path_iter.ends_with("/") {
                        full_path = path_iter.to_string() + "/" + enpoint_name;
                    } else {
                        full_path = path_iter.to_string() + enpoint_name;
                    };
    
                    /*- If the path is the same as the path we're looking for,
                        return the path -*/
                    match path {
                        RouteValue::File(file_path) => {
                            /*- Check if the path matches the one inputted -*/
                            if full_path == input_path {
                                return_value = RouteValue::File(file_path);
    
                                break 'main;
                            }
                        },
                        RouteValue::Function(func) => {
                            /*- Check if the path matches the one inputted - again... -*/
                            if full_path == input_path {
                                return_value = RouteValue::Function(*func);
    
                                break 'main;
                            }
                        },
                        RouteValue::None => {
                            if let Some(c) = options.custom404 {
                                return_value = RouteValue::File(c);
                            }else {
                                return_value = RouteValue::File("404.html");
                            };
                        },
                    };
                },
            };
        };
        
        /*- Return -*/
        return return_value;
    }
    
    /*- Send a file with its content -*/
    pub fn send_file(stream:&mut TcpStream, path:&str, static_file_path:&str) {

        /*- Get the FULL file path -*/
        let full_path = format!("{}/{}", static_file_path, path);
        
        /*- Get the file contents -*/
        let file_content = fs::read_to_string(
            &full_path
        )
        /*- We obviously don't want to panic whilst the server is
            running (will cause server to shut down), so we'll just
            send a file-not-found message if the file wasn't found -*/
        .unwrap_or(
            format!("File not found: \"{full_path}\"")
        );
    
        /*- Respond -*/
        let res:&str = &format!("HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n{}", file_content.len(), file_content);
        stream.write(res.as_bytes()).unwrap();
        stream.flush().unwrap();
    }
}