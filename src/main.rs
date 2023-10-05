use std::{io, thread};
use std::collections::HashMap;
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use tiny_http::{Request, Response, Server, StatusCode};
use flapit_server::{Message, Protocol};

fn main() -> io::Result<()> {
    eprintln!("Starting server on '{}'", "0.0.0.0:443");
    eprintln!("Starting server on '{}'", "0.0.0.0:3000");

    let listener = TcpListener::bind("0.0.0.0:443")?;
    let http = Server::http("0.0.0.0:3000").unwrap();

    let devices: Arc<Mutex<HashMap<String, TcpStream>>> = Arc::new(Mutex::new(HashMap::new()));
    let devices_clone = Arc::clone(&devices);

    thread::spawn(move || {
        for request in http.incoming_requests() {
            let devices_clone = Arc::clone(&devices);
            thread::spawn(move || -> io::Result<()> {
                let _ = handle_http(request, devices_clone).map_err(|e| eprintln!("Error: {}", e));
                Ok(())
            });
        }
    });

    for stream in listener.incoming() {
        if let Ok(stream) = stream {
            let devices_clone = Arc::clone(&devices_clone);
            thread::spawn(move || -> io::Result<()> {
                let _ = handle_connection(stream, devices_clone).map_err(|e| eprintln!("Error: {}", e));
                Ok(())
            });
        }
    }
    Ok(())
}

fn handle_http(mut request: Request, devices: Arc<Mutex<HashMap<String, TcpStream>>>) -> io::Result<()> {
    let mut content = String::new();
    request.as_reader().read_to_string(&mut content).unwrap();

    let parameters = parse_query_string(&content);

    if !parameters.contains_key("device") || !parameters.contains_key("message") {
        let response = Response::from_string("Missing \"device\" or \"message\" parameter.").with_status_code(StatusCode::from(400));
        request.respond(response)?;

        return Ok(());
    }

    let message = Message::SetCounterValue(parameters["message"].clone());

    if let Some(stream) = devices.lock().unwrap().get(parameters["device"].as_str()) {
        let mut protocol = Protocol::with_stream(stream.try_clone()?)?;
        protocol.send_message(&message)?;

        let response = Response::new_empty(StatusCode::from(202));
        request.respond(response)?;

        return Ok(());
    }

    let response = Response::from_string(format!("Device {} not found.", parameters["device"])).with_status_code(StatusCode::from(400));
    request.respond(response)?;

    Ok(())
}

fn handle_connection(stream: TcpStream, devices: Arc<Mutex<HashMap<String, TcpStream>>>) -> io::Result<()> {
    let peer_addr = stream.peer_addr()?;
    let mut protocol = Protocol::with_stream(stream.try_clone()?)?;

    loop {
        let message = protocol.read_message::<Message>()?;
        eprintln!("Incoming {:?} [{}]", message, peer_addr);

        match message {
            Message::AuthAssociate(serial, _, _) => {
                protocol.send_message(&Message::Ok())?;
                devices.lock().unwrap().insert(serial, stream.try_clone()?);
            },
            _ => ()
        }
    }
}

fn parse_query_string(string: &String) -> HashMap<String, String> {
    let mut map: HashMap<String, String> = HashMap::new();

    for parameter in string.split("&") {
        let (key, value) = parameter.split_once("=").unwrap();
        map.insert(String::from(key), String::from(value));
    }

    map
}