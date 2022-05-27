pub mod server {
    #![allow(dead_code, deprecated)]
    
    /*- Imports -*/
    use std::net::TcpListener;
    use std::net::TcpStream;
    use std::io::prelude::*;
    use std::collections::HashMap;
    use webserver::ThreadHandler;
    use std::fs;
    use termcolor::{ Color };
    use std::path::Path;
    use crate::api::utils::log;

    /*- The options that the user has before starting the server -*/
    #[derive(Clone)]
    pub struct ServerOptions {
        pub url:&'static str,
        pub port:usize,
        pub numthreads:usize,
        // pub static_files:&'static str,
        pub routes:Vec<RouteRoot>,
        pub log_status:bool,
        pub on_connect:Option<fn(&String)>,
        pub statics:Statics,
    }
    
    #[derive(Clone)]
    pub struct Statics {
        pub dir:&'static str,
        pub serve:bool,
        pub custom404:Option<&'static str>,
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
        Function(fn(TcpStream, String, HashMap<String, String>) -> ()),
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

        /*- First check if user wants to serve all static files -*/
        if options.statics.serve {
            if send_file(stream, path, options.statics.dir) == true { return; };
        };

        /*- Iterate over all of them -*/
        let value = iterate_routes(&options.routes, path, 0u8, "", &options);

        /*- Get the users prefered 404 file -*/
        let custom_404 = options.statics.custom404.unwrap_or("404.html");
    
        /*- See if the value is either a function or a file -*/
        match value.value {
            RouteValue::File(file_path) => send_file(stream, file_path, &options.statics.dir),
            RouteValue::Function(func) => return func(stream.try_clone().unwrap(), request, value.params),
            RouteValue::None => send_file(stream, custom_404, &options.statics.dir),
        };
    }

    fn remove_empty(vec:Vec<&str>) -> Vec<&str> {
        let mut new_vec = Vec::new();
        for item in vec {
            if item.len() > 0 {
                new_vec.push(item);
            }
        }
        return new_vec;
    }
    
    /*- We'll return a Routevalue and sometimes a param-map -*/
    pub struct RoutesReturn {
        value:RouteValue,
        params:HashMap<String, String>,
    }

    /*- Iterate over all of the routes to find the path's value -*/
    pub fn iterate_routes(routes:&Vec<RouteRoot>, input_path:&str, index:u8, path_iter:&str, options:&ServerOptions) -> RoutesReturn {
    
        /*- What to finnaly return -*/
        let mut return_value:RoutesReturn = RoutesReturn { value:RouteValue::None, params:HashMap::new() };
    
        /*- Iterate -*/
        'main: for route in routes.iter() {
            println!("\n\n");
            /*- Check wether the route is a stack, or a path -*/
            match route {
                RouteRoot::Stack(path,routes) => {
                    let possible_route = iterate_routes(&routes.clone(), input_path, index+1, (path_iter.to_string().clone() + *path).as_str(), &options);
                    
                    /*- If the route is a file, return it -*/
                    match possible_route.value {
                        RouteValue::File(file_path) => return_value = RoutesReturn { value:RouteValue::File(file_path), params:possible_route.params },
                        RouteValue::Function(func) => return_value = RoutesReturn { value:RouteValue::Function(func), params:possible_route.params },
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

                    /*- The map will contain the uri-variables (params). Like when connecting to test/:id -*/
                    let mut map:HashMap<String, String> = HashMap::new();

                    let mut full_iter  = remove_empty(full_path.split("/").collect::<Vec<&str>>());
                    let mut input_iter = remove_empty(input_path.split("/").collect::<Vec<&str>>());

                    /*- Add all params to the map -*/
                    for (index, param) in full_iter.clone().iter().enumerate() {
                        if param.starts_with(":") {

                            /*- Find the index of param inside of the full_iter -*/
                            let remove_index:usize = full_iter.iter().position(|&x| &x == param).unwrap();

                            /*- Insert it into the map -*/
                            map.insert(param[1..].to_string(), input_iter
                                                                        .iter()
                                                                        .nth(remove_index)
                                                                        .unwrap_or(&"null")
                                                                        .to_string());

                            /*- Remove the param from both the full_iter, and input_iter.
                                We want to do this because later when we'll check if the
                                path exists, the server will compare ex '/some/:id' with
                                '/some/hello' and won't know they're the same -*/

                            if full_iter.len() > index-1 && input_iter.len() > index-1 {

                                full_iter.remove(remove_index);
                                input_iter.remove(remove_index);

                                println!("\nremove");
                                println!("full_iter: {:?}", full_iter);
                                println!("input_iter: {:?}\n", input_iter);
                            };
                        };
                    };
    
                    /*- If the path is the same as the path we're looking for,
                        return the path -*/
                    match path {
                        RouteValue::File(file_path) => {
                            /*- Check if the path matches the one inputted -*/
                            if full_iter == input_iter {
                                return_value = RoutesReturn { value: RouteValue::File(file_path), params: map };
    
                                break 'main;
                            }
                        },
                        RouteValue::Function(func) => {
                            /*- Check if the path matches the one inputted - again... -*/
                            if full_iter == input_iter {
                                return_value = RoutesReturn { value: RouteValue::Function(*func), params: map };
    
                                break 'main;
                            }
                        },
                        RouteValue::None => {
                            if let Some(c) = options.statics.custom404 {
                                return_value = RoutesReturn { value: RouteValue::File(c), params: map };
                            }else {
                                return_value = RoutesReturn { value: RouteValue::File("404.html"), params: map };
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
    pub fn send_file(stream:&mut TcpStream, path:&str, static_file_path:&str) -> bool {

        /*- Get the FULL file path -*/
        let full_path = format!("{}/{}", static_file_path, path);

        /*- Check if the file exists -*/
        if !Path::new(&full_path).is_file() { return false; };

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

        /*- Return success -*/
        true
    }
}