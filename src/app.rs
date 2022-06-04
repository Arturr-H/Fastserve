pub mod server {
    #![allow(dead_code, deprecated)]
    
    /*- Imports -*/
    use std::net::TcpListener;
    use std::net::TcpStream;
    use std::io::prelude::*;
    use std::collections::HashMap;
    use super::webhandler::ThreadHandler;
    use std::fs;
    use termcolor::{ Color };
    use std::path::Path;
    use crate::app::utils::log;

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
                            if full_iter.len() > index-1 && input_iter.len() > remove_index {
                                full_iter .remove(remove_index);
                                input_iter.remove(remove_index);
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

/*- Put all general-purpose functions here, like
    parsing headers, sending repsonses and more -*/
pub(crate) mod utils {
    #![allow(deprecated)]

    use std::collections::HashMap;
    use std::net::TcpStream;
    use std::io::Write;
    use lazy_static::lazy_static;
    use std::hash::{ Hash, Hasher };
    use std::collections::hash_map::DefaultHasher;
    use termcolor::{ Color, ColorChoice, ColorSpec, StandardStream, WriteColor };

    /*- Static mutable variables -*/
    lazy_static! {
        static ref STATUS_CODES:&'static [(&'static u16, &'static str); 58] = &[
        (&400, "Bad Request"),                      (&500, "Internal Server Error"),
        (&401, "Unauthorized"),                     (&501, "Not Implemented"),
        (&402, "Payment Required"),                 (&502, "Bad Gateway"),
        (&403, "Forbidden"),                        (&503, "Service Unavailable"),          /*=-----------=*/
        (&404, "Not Found"),                        (&504, "Gateway Timeout"),              //             \\
        (&405, "Method Not Allowed"),               (&505, "HTTP Version Not Supported"),   //     500     \\
        (&406, "Not Acceptable"),                   (&506, "Variant Also Negotiates"),      //             \\
        (&407, "Proxy Authentication Required"),    (&507, "Insufficient Storage"),         /*=-----------=*/
        (&408, "Request Timeout"),                  (&508, "Loop Detected"),
        (&409, "Conflict"),                         (&510, "Not Extended"),
        (&410, "Gone"),                             (&511, "Network Authentication Required"),
        (&411, "Length Required"),                              (&200, "OK"),
        (&412, "Precondition Failed"),                          (&201, "Created"),
        (&413, "Payload Too Large"),           /* 200 OK -> */  (&202, "Accepted"),
        (&414, "URI Too Long"),                /* 200 OK -> */  (&204, "No Content"),
        (&415, "Unsupported Media Type"),      /* 200 OK -> */  (&205, "Reset Content"),
        (&416, "Range Not Satisfiable"),       /* 200 OK -> */  (&206, "Partial Content"),
        (&417, "Expectation Failed"),          /* 200 OK -> */  (&207, "Multi-status"),
        (&418, "I'm a teapot"),                                 (&208, "Already reported"), 
        (&421, "Misdirected Request"),                          (&226, "IM Used"),
        (&422, "Unprocessable Entity"),             (&300, "Multiple Choices"),
        (&423, "Locked"),                           (&301, "Moved Permanently"),
        (&424, "Failed Dependency"),                (&302, "Found"),                    /*=-----------=*/
        (&425, "Too Early"),                        (&303, "See Other"),                //             \\
        (&426, "Upgrade Required"),                 (&304, "Not Modified"),             //     300     \\
        (&428, "Precondition Required"),            (&305, "Use Proxy"),                //             \\
        (&429, "Too Many Requests"),                (&306, "Switch Proxy"),             /*=-----------=*/
        (&431, "Request Header Fields Too Large"),  (&307, "Temporary Redirect"),
        (&451, "Unavailable For Legal Reasons"),    (&308, "Permanent Redirect"),


        ];
    }

    /*- The get_header function can either take
    multiple header requests as input, or just one -*/
    #[derive(Debug)]
    #[allow(dead_code)]
    pub enum HeaderReturn<'l> {
        Single(&'l str),
        Multiple(Vec<&'l str>),
        Values(HashMap<&'l str, &'l str>),
        All,
        None,
    }

    /*- Get the request headers -*/
    pub fn parse_headers<'a>(request:&'a str, header:HeaderReturn<'a>) -> HeaderReturn<'a> {
        /*- Get the headers from the request -*/
        let header_strings = request.split("\n");

        /*- A hasmap of all heders -*/
        let mut headers:HashMap<&str, &str> = HashMap::new();

        /*- We want to split by the first colon, if we
            don't do that and users start inputting colons
            in their header string, it'll lead to an error -*/
        for header in header_strings {

            /*- Loop through every char -*/
            'charLoop: for (i, c) in header.chars().enumerate() {
                /*- If we find a colon, split the string -*/
                if c == ':' {
                    /*- Get the key and value -*/
                    let key = &header[0..i];
                    let value = &header[i+1..];

                    /*- Add the key and value to the headers -*/
                    headers.insert(key, value);

                    /*- Break out of the loop -*/
                    break 'charLoop;
                };
            };
        };

        /*- See what type of header the user wants -*/
        match header {
            HeaderReturn::Single(v) => {
                return HeaderReturn::Single(
                    headers.get(v).unwrap_or(&"").trim()
                );
            },
            HeaderReturn::Multiple(v) => {

                /*- Header "queue" -*/
                let headers_to_get:Vec<&str> = v.into_iter().collect::<Vec<&str>>();

                /*- Return the headers -*/
                return HeaderReturn::Multiple(
                    headers_to_get.into_iter().map(|v| {
                        headers.get(v).unwrap_or(&"").trim()
                    }).collect::<Vec<&str>>()
                );
            },
            HeaderReturn::All => {
                return HeaderReturn::Values(
                    /*- Trim all values -*/
                    headers.into_iter().map(|(k, v)| {
                        (k, v.trim())
                    }).collect::<HashMap<&str, &str>>()
                );
            },
            HeaderReturn::Values(_) => panic!("Can't get values - Values is read only"),
            HeaderReturn::None =>      panic!("Can't get None - None is read only"),
        };
    }

    /*- What we want to respond with -*/
    pub enum ResponseType {
        Text,
        Json,
        Html,
    }

    /*- Return a http response containing the status -*/
    pub fn respond(
        stream:&mut TcpStream,
        status:u16,
        response_type:Option<ResponseType>,
        content:Option<&str>
    ) -> () {

        /*- Get the status string -*/
        let status_msg = STATUS_CODES.iter().find(|&x| x.0 == &status).unwrap_or(&(&0u16, "Internal error - Missing status code")).1;

        /*- Get the response type -*/
        let response_type = match response_type {
            Some(ResponseType::Text) => "text/plain",
            Some(ResponseType::Json) => "application/json",
            Some(ResponseType::Html) => "text/html",
            None => "text/plain",
        };

        /*- Get the content exists -*/
        if let Some(c) = content {
            /*- Write the status to the stream -*/
            stream.write(
                format!("HTTP/1.1 {}\r\nContent-Length: {}\r\nContent-Type: {}\r\n\r\n{}", status, c.len(), response_type, c).as_bytes()
            ).unwrap();
        }else {
            /*- Write the status to the stream -*/
            stream.write(
                format!("HTTP/1.1 {}\r\n\r\n{} {}", status, status, status_msg).as_bytes()
            ).unwrap();
        };

        /*- Flush the stream -*/
        stream.flush().unwrap();
    }

    /*- Quick function to respond with a message
    saying that some headers might be missing -*/
    pub fn expect_headers(
        stream:&mut TcpStream,
        headers:&HeaderReturn,
        required:Vec<&str>,
    ) -> bool {

        /*- Quick way of sending a message of which headers that are missing -*/
        fn get_missing_response(missing:&Vec<&str>) -> String { format!("Missing headers: {:?}", missing) }

        /*- Check what type of headers we got as input -*/
        match headers {
            HeaderReturn::Values(v) => {
                for request in &required {
                    if !v.contains_key(request) {
                        respond(stream, 400, Some(ResponseType::Text), Some(&get_missing_response(&required)));
                        return false;
                    };
                };
            },
            _ => panic!("Expected HeaderReturn::Values"),
        };

        /*- Return that all headers are specified -*/
        true
    }

    /*- Because when we change the terminal color, 
        it will keep the same color for future lines -*/
    fn reset_terminal_color(stdout: &mut StandardStream) {
        stdout.set_color(
            ColorSpec::new()
                .set_fg(Some(Color::Rgb(171, 178, 191))))
                .unwrap();
    }

    /*- Print a response with colors -*/
    pub fn log(clr:Color, msg:&str) {
        /*- Set new standard output -*/
        let mut stdout = StandardStream::stdout(ColorChoice::Always);

        /*- Set the color to the inputted one -*/
        stdout.set_color(
            ColorSpec::new()
                .set_fg(Some(clr)))
                .unwrap();

        /*- Print it -*/
        writeln!(&mut stdout, "{}", msg).unwrap();

        /*- Reset the color -*/
        reset_terminal_color(&mut stdout);
    }

    /*- Hash input -*/
    pub(crate) fn hash<H: Hash>(hashval:&H,u:bool) -> String {
        /*- Initialize the hasher -*/
        let mut hasher = DefaultHasher::new();

        /*- Hash the string and end it -*/
        hashval.hash(&mut hasher);

        /*- Make it hex -*/
        if u { return format!("{:X}", hasher.finish()); }
        else { return format!("{:x}", hasher.finish()); }
    }
}

/*- Handle requests concurrently -*/
pub(crate) mod webhandler {
    #![allow(dead_code)]

    /*- Imports -*/
    use std::{ sync::mpsc, thread };
    use std::sync::mpsc::Receiver;
    use std::sync::{ Arc, Mutex };
    use std::thread::JoinHandle;

    pub struct ThreadHandler {
        workers: Vec<Worker>,
        sender: mpsc::Sender<Job>
    }

    type Job = Box<dyn FnOnce() + Send + 'static>;

    impl ThreadHandler {

        /*- Create a new ThreadHandler -*/
        /*- num_threads must be greater than 0 -*/
        pub fn new(num_threads:usize) -> ThreadHandler {
            assert!(num_threads > 0);

            let (sender, reciever) = mpsc::channel();
            let reciever:Arc<Mutex<Receiver<Job>>> = Arc::new(Mutex::new(reciever));
            let mut workers:Vec<Worker> = Vec::with_capacity(num_threads);

            /*- Give the workers their tasks -*/
            for id in 0..num_threads {
                workers.push(Worker::new(id, Arc::clone(&reciever)));
            }
            ThreadHandler { workers, sender }
        }

        pub fn exec<F>(&self, f:F) where 
            F:FnOnce() + Send + 'static
        {
            let job:Box<F> = Box::new(f);
            self.sender.send(job).unwrap();
        }
    }

    struct Worker {
        id:usize,
        thread:thread::JoinHandle<()>
    }

    impl Worker {
        fn new(id:usize, reciever:Arc<Mutex<mpsc::Receiver<Job>>>) -> Worker {
            let thread:JoinHandle<()> = std::thread::spawn(move || loop {

                /*- Get a job -*/
                let job:Box<dyn FnOnce() + Send> = reciever.lock().unwrap().recv().unwrap();

                job();
            });

            /*- Return the id and the thread that the worker is using -*/
            Worker { id, thread }
        }
    }
}