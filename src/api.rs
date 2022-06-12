/*- Put all endpoint-functions in here -*/
pub mod functions {
    /*- "Rules" -*/
    #![allow(dead_code, unused_imports)]

    /*- Imports -*/
    use std::net::TcpStream;
    use std::collections::HashMap;
    use std::fs;
    use std::path::Path;
    use crate::global::{ PORT, URL, VISIBLE, PROTOCOL };
    use crate::server::utils::{
        expect_headers, respond, parse_headers,
        hash, ResponseType, HeaderReturn
    };

    /*- Constants -*/
    const MONGO_URI:&str = "mongodb://localhost:27017";
    const FILE_B64:&str = "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAACgAAAAqCAYAAADBNhlmAAAEe0lEQVRYR+2YDUxbVRTH/6+sxQ4Lgrq5DZYhyzKUdJKCSFY+iqJzZp0aEfwg2RYXnAkTWHVxU+McRJ1bnTIyIHHix6JbXXSJLs7PqotCpMKYBWEooHzokI11DCjQXs8jXdzE9t3u1UlMb/LyXptzz/3d/zv33POugGnehGnOhyCg3DcUVDCoII8CjDHBsHpLRLdmYKS9vNzJ04fXxr8Y3M6OYgbmkfPL4QaDAiF0n6Dfoh+F53Kg9YN8VK04xAvhy44PsOCdJYjPtRLSSQzDidHR3wH3GGFRf8VMuF0RUIUlEPZf7ef6jdiVvE0uJB/gM+5xhAt2NO7bjTfyqv4+qE6nW9wTs+yx39JL1+AEsUfMBtRw4ae6PJTf9K4cSB5ABXYwF73Mt1Ek3O9tsLnGTWt6M8teRduXPy5sf+3T9uyadQiFA521S7EzteViIXkAATNjBGjGo8IGbwPNWbF5U5+htAzt1g91daZVjqiU6OPZFWaoEIcl1jgYDGKs+t0kAen1KW0P1I+R5xKUCC95BTQ+vbYvc0s12r56D5UZd4t2OTn7VZaknEqCzECxEOc3nWf1+eyXkpISXpdbe5qM7iTAg96M5xk3P9mTWboVx60HsNtwzwV229hWgkynEMnwF1JSwcTExLkN+d/XQkMqrBU6vMbgyqce6c14tgJt1r2oNDw4xe5FdgeFSRVNMtofSElArVYb27Tq6CECXO4LkJK1ggKAVq51ByoMpn+E2MUWYQytBCk57rn+koYUg3MoBj+XApx0aGZDGHD0Rbcd1i5Ef8iZyBluW3XB8AWwZjZMyf0ETMICHiUlAQsLC0PLY19p5AJ8npVRrOXTniKmbErglGgoo9P1K/1no6dYek6cBONUURLQo0wLD6D2rodnNWmy0nFZVBoEIWqyLxPOEGM3FAobKrMOYz9ToZt2o/8C8PxXZjQaNV1dXeGCIMxUq9VsdHR0vKGhocszYRZowAFagS9Qopa9t2Jjy22YvfijQAOKlUsN5bHVPIHt06bk2MuITlgfaEAHKbiXFFwnG7C4+X3ExK8MLOBOJu4kD5GCFvmAxz5BTMItgQU0M1FBEylYLRuwyP415l+nDzSgmGwPktP7pisgo9rZThVJgnzA5m8wPz410AqKq3iQYjAyAIAfE2D2vwHo0r6ZtrTJdqROFmRx8z5axbcSINdkebc6UcH+pLey0urrv2iVBVj0w+uIud6NDXw5lQ9wO5X8CjTnfHuv1mKxuC4WUK/XRx5JO1CDq2ft8VX8nu9fEjA5Ofma75Z91oNIzQg67VW6jrLn0DtEPgaV40LIlUrmnFCGKYWxIVcE/T6rVrudw+NRp0MnTimUmjCxmsHQiGpEpYam46qbk07p12fQ5+sTtOAGeSYqCUj1YNwv1+bd3p9qKp/iUEFDuacc4YkfR+LpgniJz2dJ/ZN07yJrO93NvHDieJKA4rEGqXhD54LcG/9YVODEFZpGPC408sw+EDaSgJ5BxGMNsfC85I0X8JKDnRswCChX+qCC/3sF/wTGeYE6vRMEVQAAAABJRU5ErkJggg==";
    const FOLDER_B64:&str = " data:image/png;base64,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";
    const IMPORTANT_NAMES:[&str; 4] = ["Desktop", "rust", "Documents", "Applications"];

    pub fn test_fn(mut stream:TcpStream, request:String, _params:HashMap<String, String>) {

        /*- Get the headers -*/
        let headers = parse_headers(&request, HeaderReturn::All);

        /*- Check if all headers are specified -*/
        expect_headers(&mut stream, &headers, vec!["Hosta"]);
        if let HeaderReturn::Values(c) = headers {
            respond(&mut stream, 200, Some(ResponseType::Json), Some(format!("{:?}", c).as_str()));
        };
    }

    pub fn google_test(mut stream:TcpStream, _request:String, params:HashMap<String, String>) {
        let response = reqwest::blocking::get(
            format!("https://{}",
                params.get("url").unwrap_or(&String::from(""))
            )
        ).unwrap();

        respond(&mut stream, 200, Some(ResponseType::Html), Some(&response.text().unwrap()
            .replace("<body", "<body style='filter: blur(2px)'")
        ));
    }

    pub fn param_test(mut stream:TcpStream, _request:String, params:HashMap<String, String>) {
        respond(&mut stream, 200, Some(ResponseType::Text), Some(
            &format!(
                "param1: {}, param2: {} \n hash:{}",
                params.get("url").unwrap(),
                params.get("shit").unwrap(),
                hash(&params.get("url"), false)
            )
        ));
    }

    pub fn get_artur_dir(mut stream:TcpStream, _:String, params:HashMap<String, String>) {

        /*- Get the path the user wants to see -*/
        let local_path:&str = params.get("local").unwrap();

        /*- Because we can't use forward slashes in url:s -*/
        let local_path:&str = &format!("/{}", local_path.replace(":", "/").replace("%20", " "));

        /*- If it's a file -*/
        if Path::new(local_path).is_file() {
            let file_content:&str = &fs::read_to_string(local_path).unwrap_or("".to_string());
            return respond(&mut stream, 200, Some(ResponseType::Text), Some(file_content));
        }

        /*- Read the path -*/
        let p:fs::ReadDir;
        if Path::new(&local_path).is_dir() {
            p = fs::read_dir(local_path).unwrap();
        }else {
            return respond(&mut stream, 404, None, None);
        }

        /*- Where we'll put all html data -*/
        let mut end_html:String = String::new();
        let mut end_content:Vec<String> = Vec::new();
        for a in p { end_content.push(a.unwrap().path().display().to_string()); }
        end_content.sort();

        for a in end_content.iter() {
            /*- The path we are currently on -*/
            let current_path:&str = &a;

            /*- If the path is a directory -*/
            let is_dir:&bool = &Path::new(a).is_dir();
            let image:&str = if *is_dir { FOLDER_B64 } else { FILE_B64 };

            /*- Get the output-html-block -*/
            let string:&str = &format!(
                "<div style='display:flex;align-items:center;margin-bottom:12px;'><img src='{image}'><a href='{PROTOCOL}://{VISIBLE}/artur/{}'>{}</a></div>",//{URL}:{PORT}
                &current_path.replace("/", ":")[1..current_path.len()], // - the url
                current_path
            );

            /*- Add the block to the html-string -*/
            end_html += string;
        }

        respond(&mut stream, 200, Some(ResponseType::Html), Some(&end_html));
    }

    pub fn terminal(mut stream:TcpStream, _:String, params:HashMap<String, String>) -> () {

        /*- Get the command the user wants to execute -*/
        let command:&str = &params.get("command").unwrap();

        /*- Convert it to a non-url-command -*/
        let command:&str = &command.replace("%20", " ");

        /*- Execute it -*/
        let output:&str = &String::from_utf8(
            std::process::Command::new("sh")
                .arg("-c")
                .arg(command)
                .output()
                .unwrap().stdout)
            .unwrap_or(
                "".to_string()
        );
        

        respond(&mut stream, 200, Some(ResponseType::Text), Some(&output));
    }
}