#![allow(deprecated, dead_code)]

use std::collections::HashMap;
use std::net::TcpStream;
use std::io::Write;
use std::path::Path;
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
#[derive(Debug)]
pub enum ResponseType {
    Text,
    Json,
    Html,
    Image(ResponseTypeImage)
}
#[derive(Debug)]
pub enum ResponseTypeImage {
    Jpeg,
    Png,
    Gif,
    Webp,
    Svg,
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
        Some(ResponseType::Image(c)) => {
            match c {
                ResponseTypeImage::Jpeg => "image/jpeg",
                ResponseTypeImage::Png => "image/png",
                ResponseTypeImage::Gif => "image/gif",
                ResponseTypeImage::Webp => "image/webp",
                ResponseTypeImage::Svg => "image/svg+xml",
            }
        }
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

pub fn guess_response_type(path:&str) -> ResponseType {
    let path:&Path = Path::new(path);

    /*- Check extensions -*/
    match path.extension() {
        Some(ext) => {
            match ext.to_str() {
                /*- Html -*/
                Some("html") => return ResponseType::Html,
                Some("htm")  => return ResponseType::Html,

                /*- Json -*/
                Some("json") => return ResponseType::Json,
                Some("yml")  => return ResponseType::Json,
                Some("yaml") => return ResponseType::Json,

                /*- Images -*/
                Some("gif")  => return ResponseType::Image(ResponseTypeImage::Gif),
                Some("png")  => return ResponseType::Image(ResponseTypeImage::Png),
                Some("jpg")  => return ResponseType::Image(ResponseTypeImage::Jpeg),
                Some("jpeg") => return ResponseType::Image(ResponseTypeImage::Jpeg),
                Some("webp") => return ResponseType::Image(ResponseTypeImage::Webp),
                Some("svg")  => return ResponseType::Image(ResponseTypeImage::Svg),
 
                /*- Text -*/
                Some(_)   => return ResponseType::Text,
                None      => return ResponseType::Text,
            };
        },
        None => return ResponseType::Text,
    };
}
