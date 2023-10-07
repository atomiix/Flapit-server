use std::{io};
use std::cmp::{min};
use std::io::{BufRead, Read, Write};
use std::net::TcpStream;
use byteorder::{NetworkEndian, ReadBytesExt};
use regex::Regex;
use crate::Message::{AuthAssociate, Echo, Idle, SetCounterValue};

pub trait Deserialize {
    type Output;

    fn deserialize(buf: &mut (impl Read + BufRead)) -> io::Result<Self::Output>;
}

pub trait Serialize {
    fn serialize(&self, buf: &mut impl Write) -> io::Result<()>;
}

pub struct Protocol {
    reader: io::BufReader<TcpStream>,
    stream: TcpStream,
}

impl Protocol {
    pub fn with_stream(stream: TcpStream) -> io::Result<Self> {
        Ok(Self {
            reader: io::BufReader::new(stream.try_clone()?),
            stream,
        })
    }

    pub fn read_message<T: Deserialize>(&mut self) -> io::Result<T::Output> {
        T::deserialize(&mut self.reader)
    }

    pub fn send_message(&mut self, message: &impl Serialize) -> io::Result<()> {
        message.serialize(&mut self.stream)?;
        self.stream.flush()
    }
}

#[derive(Debug)]
pub enum Message {
    Authenticate(),
    Ok(),
    Echo(),
    AuthAssociate(String, String, String),
    SetCounterValue(String),
    Idle(),
}

impl Deserialize for Message {
    type Output = Message;

    fn deserialize(buf: &mut (impl Read + BufRead)) -> io::Result<Self::Output> {
        let size = buf.read_u32::<NetworkEndian>()?;
        let message_type = buf.read_u32::<NetworkEndian>()?;

        let mut data = 4;
        let mut args: Vec<String> = vec![String::new(); 3];

        let mut index = 0;
        while data < size {
            let string_len = buf.read_u32::<NetworkEndian>()?;
            let mut string_buf = vec![0u8; string_len as usize];
            buf.read_exact(&mut string_buf)?;
            args[index] = String::from_utf8(string_buf).unwrap();
            data += string_len + 4;
            index += 1;
        }

        match message_type {
            4 => Ok(Echo()),
            5 => Ok(AuthAssociate(args[0].clone(), args[1].clone(), args[2].clone())),
            6 => Ok(Idle()),
            _ => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Invalid Request Type",
            ))
        }
    }
}

impl Serialize for Message {
    fn serialize(&self, buf: &mut impl Write) -> io::Result<()> {
        match self {
            SetCounterValue(message) => {
                let flaps = flaps_from_string(message);
                buf.write_all(&flaps)?
            },
            Echo() => {
                buf.write_all(&[0, 0, 0, 4, 0, 0, 0, 4])?
            },
            Message::Ok() => {
                buf.write_all(&[0, 0, 0, 4, 0, 0, 0, 1])?
            },
            _ => ()
        }
        Ok(())
    }
}

const LEAD_FLAPS: [&str; 60] = ["BLANK", "BLAST", "LIKE_RU", "7", "LIKE_FR", "LIKE_US_EN",
    "LIKE_EN", "LIKE_JA", "LIKE_KO", "8", "LIKE_ZH", "FOLLOW_US_ES", "FOLLOW_US_PT", "FOLLOW_US_RU",
    "FOLLOW_US_FR", "9", "FOLLOW_US_EN", "FOLLOW_US_IT", "FOLLOW_US_DE", "FOLLOW_US_JP",
    "FOLLOW_US_KO", "FOLLOW_US_ZH", "CHECK_IN_PT", "CHECK_IN_RU", "CHECK_IN_JP", "CHECK_IN_KO",
    "CHECK_IN_ZH", "CHECK_IN_EN", "1", "#", "TWITTER", "ZOMATO", "YOUTUBE", "2", "INSTAGRAM",
    "GOOGLE_STATS", "VKONTAKTE", "FOURSQUARE", "SWARM", "3", "YELP", "TRIPADVISOR", "WEIBO",
    "DIANPING", "FACEBOOK", "4", "SMILEY", "FACEBOOK_THUMB", "THANKS_ES", "THANKS_PT", "THANKS_RU",
    "5", "THANKS_FR", "THANKS_EN", "THANKS_IT", "THANKS_DE", "THANKS_JA", "6", "THANKS_KO",
    "THANKS_ZH"];

const FLAPS: [&str; 60] = ["BLANK", "EMPTY_BUBBLE", "1", "A", "B", "C", "D", "/", "E", "F", "G",
    "H", "I", "2", "J", "K", "L", "M", "N", "5", "O", "P", "Q", "R", "S", "3", "T", "U", "V", "W",
    "X", "6", "Z", "Y", "HEART", "€", "$", "4", "£", "¥", "+", "?", "!", "FULL_BUBBLE", "&", "@",
    "#", "->", ":", "0", ".", "9", "HALF_BUBBLE", "FULL_STAR", "HALF_STAR", "EMPTY_STAR", "BLANK2",
    "8", "PERCENT", "7"];

fn flaps_from_string(message: &String) -> [u8;15] {

    let mut payload = [0, 0, 0, 11, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0];

    let (lead, flaps) = match message.split_once(" ") {
        Some((lead, flaps)) => (lead, flaps),
        _ => ("blank", message.as_str())
    };

    let pattern = &mut FLAPS.iter().map(|word| -> String { regex::escape(word) }).collect::<Vec<_>>();
    pattern.sort_by(|a, b| Ord::cmp(&b.len(), &a.len()));
    let regex = Regex::new(&format!(r"({})", &pattern.join("|"))).unwrap();

    let flaps = &flaps.to_uppercase();
    let matches: Vec<_> = regex.find_iter(flaps).collect();

    let lead = &lead.to_uppercase();
    payload[8] = match LEAD_FLAPS.iter().position(|&e| lead.as_str() == e) {
        None => 0,
        Some(index) => index as u8
    };

    for (i, m) in matches[..min(6, matches.len())].iter().enumerate() {
        payload[i+9] = FLAPS.iter().position(|&e| m.as_str() == e).unwrap() as u8;
    }

    println!("Sending {} {}", LEAD_FLAPS[payload[8] as usize], payload.map(|v| FLAPS[v as usize])[9..].join("|"));

    payload
}
