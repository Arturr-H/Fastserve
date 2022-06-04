/*- Put all endpoint-functions in here -*/
pub mod functions {
    /*- "Rules" -*/
    #![allow(redundant_semicolons)]

    /*- Imports -*/
    use std::net::TcpStream;
    use std::collections::HashMap;
    use crate::app::utils::{
        expect_headers, respond, parse_headers,
        hash, ResponseType, HeaderReturn
    };
    use mongodb::{
        bson::doc,
        sync::Client,
    };

    /*- Constants -*/
    const MONGO_URI:&str = "mongodb://localhost:27017";

    /*- Quick way of initializing the mongodb client -*/
    fn initialize_client() -> mongodb::sync::Database {
        let client:mongodb::sync::Client = Client::with_uri_str(MONGO_URI).expect("Failed to initialize client");
        return client.database("db");
    }
    
    pub fn test_fn(mut stream:TcpStream, request:String, _params:HashMap<String, String>) {

        /*- Get the headers -*/
        let headers = parse_headers(&request, HeaderReturn::All);

        /*- Check if all headers are specified -*/
        expect_headers(&mut stream, &headers, vec!["Hosta"]);
        if let HeaderReturn::Values(c) = headers {
            respond(&mut stream, 200, Some(ResponseType::Json), Some(format!("{:?}", c).as_str()));
        };
    }

    pub fn get_all_users(mut stream:TcpStream, _:String, _params:HashMap<String, String>) {

        let db:mongodb::sync::Database = initialize_client();
        let users = db.collection::<mongodb::bson::Document>("users").find(None, None).unwrap();
        let mut users_vec:Vec<String> = Vec::new();
        for user in users {
            users_vec.push(format!("{:?}", user).as_str().to_string());
        };
        respond(&mut stream, 200, Some(ResponseType::Json), Some(format!("{:?}", users_vec).as_str()));
    }

    pub fn insert_user(mut stream:TcpStream, _request:String, _params:HashMap<String, String>) {

        let db:mongodb::sync::Database = initialize_client();
        let users = db.collection::<mongodb::bson::Document>("users");
        let user = doc! { "name": "Bob" };

        users.insert_one(user, None).unwrap();

        respond(&mut stream, 200, None, None);
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
}