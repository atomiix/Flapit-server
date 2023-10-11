use std::{io, thread};
use std::collections::HashMap;
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};
use clap::Parser;
use tiny_http::{Request, Response, Server, StatusCode};
use flapit_server::{Message, Protocol};
use flapit_server::Message::{Echo};

fn main() -> io::Result<()> {
    let args = Args::parse();
    eprintln!("Starting API server on '0.0.0.0:{}'", args.api_port);
    eprintln!("Starting device server on '0.0.0.0:{}'", args.device_port);

    let listener = TcpListener::bind(format!("0.0.0.0:{}", args.device_port))?;
    let http = Server::http(format!("0.0.0.0:{}", args.api_port)).unwrap();

    let devices: Arc<Mutex<HashMap<String, (SystemTime, TcpStream)>>> = Arc::new(Mutex::new(HashMap::new()));
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

fn handle_http(mut request: Request, devices: Arc<Mutex<HashMap<String, (SystemTime, TcpStream)>>>) -> io::Result<()> {
    let mut content = String::new();
    request.as_reader().read_to_string(&mut content).unwrap();

    let parameters = parse_query_string(&content);

    if !parameters.contains_key("device") || !parameters.contains_key("message") {
        let response = Response::from_string("Missing \"device\" or \"message\" parameter.").with_status_code(StatusCode::from(400));
        request.respond(response)?;

        return Ok(());
    }

    let message = Message::SetCounterValue(parameters["message"].clone());

    if let Some((_, stream)) = devices.lock().unwrap().get(parameters["device"].as_str()) {
        let response = Response::new_empty(StatusCode::from(202));
        request.respond(response)?;

        let mut protocol = Protocol::with_stream(stream.try_clone()?)?;
        if protocol.send_message(&message).is_err() {
            devices.lock().unwrap().remove(parameters["device"].as_str());
            println!("{} Removed!", parameters["device"].as_str());
        }

        return Ok(());
    }

    let response = Response::from_string(format!("Device {} not found.", parameters["device"])).with_status_code(StatusCode::from(400));
    request.respond(response)?;

    Ok(())
}

fn handle_connection(stream: TcpStream, devices: Arc<Mutex<HashMap<String, (SystemTime, TcpStream)>>>) -> io::Result<()> {
    stream.set_read_timeout(Some(Duration::from_secs(60)))?;
    let peer_addr = stream.peer_addr()?;
    let mut protocol = Protocol::with_stream(stream.try_clone()?)?;
    let mut serial: Option<String> = None;
    let handle_time = SystemTime::now();

    loop {
        let message = match protocol.read_message::<Message>() {
            Ok(m) => m,
            Err(_) => {
                eprintln!("Sending Echo");
                if protocol.send_message(&Echo()).is_ok() {
                    if let Ok(message) = protocol.read_message::<Message>() {
                        message
                    } else {
                        break
                    }
                } else {
                    break
                }
            }
        };
        eprintln!("Incoming {:?} [{}]", message, peer_addr);

        match message {
            Message::AuthAssociate(s, _, _) => {
                protocol.send_message(&Message::Ok())?;
                serial = Some(s.clone());
                devices.lock().unwrap().insert(s, (handle_time, stream.try_clone()?));
            },
            _ => ()
        }
    }
    if serial.is_some() {
        let mut devices = devices.lock().unwrap();
        match devices.get(&serial.clone().unwrap()) {
            Some((saved_time, _)) => {
                if saved_time == &handle_time {
                    devices.remove(&serial.clone().unwrap());
                    println!("{} Removed!", serial.unwrap());
                }
            },
            None => ()
        }
    }
    Ok(())
}

fn parse_query_string(string: &String) -> HashMap<String, String> {
    let mut map: HashMap<String, String> = HashMap::new();

    for parameter in string.split("&") {
        if let Some((key, value)) = parameter.split_once("=") {
            map.insert(String::from(key), String::from(value));
        }
    }

    map
}

#[derive(Parser, Debug)]
struct Args {
    #[arg(short, long, default_value_t=3000)]
    api_port: u16,

    #[arg(short, long, default_value_t=443)]
    device_port:u16,
}
