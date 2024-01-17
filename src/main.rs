slint::include_modules!();

mod store;

use crate::store::StoreBuilder;
use chrono::Utc;
use csv;
use dirs::public_dir;
use log::LevelFilter;
use rosc::{OscPacket, OscType};
use serde_json::json;
use slint::{ModelRc, VecModel};
use std::net::UdpSocket;
use std::process::Command;
use std::rc::Rc;
use std::str::FromStr;
use std::thread;
use std::{env, fs, path::PathBuf};
use strum::IntoEnumIterator;

#[derive(Debug, strum_macros::EnumIter)]
pub enum Languages {
    EN,
    JP,
    CN,
    KR,
    ES,
    FR,
    IT,
    DE,
    RU,
    PL,
}

impl Languages {
    pub fn value(&self) -> &'static str {
        match self {
            Languages::EN => "xkb:us::eng",
            Languages::JP => "anthy",
            Languages::CN => "libpinyin",
            Languages::KR => "hangul",
            Languages::ES => "xkb:es::spa",
            Languages::FR => "xkb:fr::fra",
            Languages::IT => "xkb:it::ita",
            Languages::DE => "xkb:de::deu",
            Languages::RU => "xkb:ru::rus",
            Languages::PL => "xkb:pl::pol",
        }
    }

    pub fn from_value(value: &str) -> Option<Languages> {
        match value {
            "xkb:us::eng" => Some(Languages::EN),
            "anthy" => Some(Languages::JP),
            "libpinyin" => Some(Languages::CN),
            "hangul" => Some(Languages::KR),
            "xkb:es::spa" => Some(Languages::ES),
            "xkb:fr::fra" => Some(Languages::FR),
            "xkb:it::ita" => Some(Languages::IT),
            "xkb:de::deu" => Some(Languages::DE),
            "xkb:ru::rus" => Some(Languages::RU),
            "xkb:pl::pol" => Some(Languages::PL),
            _ => None,
        }
    }
}

impl FromStr for Languages {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "EN" => Ok(Languages::EN),
            "JP" => Ok(Languages::JP),
            "CN" => Ok(Languages::CN),
            "KR" => Ok(Languages::KR),
            "ES" => Ok(Languages::ES),
            "FR" => Ok(Languages::FR),
            "IT" => Ok(Languages::IT),
            "DE" => Ok(Languages::DE),
            "RU" => Ok(Languages::RU),
            "PL" => Ok(Languages::PL),
            _ => Err(()),
        }
    }
}

impl ToString for Languages {
    fn to_string(&self) -> String {
        match self {
            Languages::EN => "EN".to_string(),
            Languages::JP => "JP".to_string(),
            Languages::CN => "CN".to_string(),
            Languages::KR => "KR".to_string(),
            Languages::ES => "ES".to_string(),
            Languages::FR => "FR".to_string(),
            Languages::IT => "IT".to_string(),
            Languages::DE => "DE".to_string(),
            Languages::RU => "RU".to_string(),
            Languages::PL => "PL".to_string(),
        }
    }
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
struct Row {
    language: String,
    sentence: String,
    timestamp: String,
}

fn count_csv_rows(file_path: &PathBuf) -> usize {
    let rdr = csv::Reader::from_path(&file_path);
    if let Ok(mut rdr) = rdr {
        if let Ok(records) = rdr.records().collect::<Result<Vec<_>, _>>() {
            return records.len();
        }
    }
    0
}

fn get_new_filename(base_dir: &PathBuf) -> PathBuf {
    let mut last_csv_idx = 0;
    fs::create_dir_all(base_dir.join("sentences")).unwrap();

    // List all csv files in the directory
    fs::read_dir(base_dir.join("sentences"))
        .unwrap()
        .for_each(|entry| {
            let entry = entry.unwrap();
            last_csv_idx = std::cmp::max(
                entry
                    .path()
                    .file_stem()
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .parse::<usize>()
                    .unwrap(),
                last_csv_idx,
            );
        });

    base_dir
        .join("sentences")
        .join(format!("{}.csv", last_csv_idx + 1))
}

fn remove_file_if_exists(file_path: &PathBuf) {
    log::info!("Removing file {}", file_path.display());
    if file_path.exists() {
        fs::remove_file(file_path).unwrap_or_else(|e| {
            log::error!("Error removing file {}: {}", file_path.display(), e);
        });
    } else {
        log::info!("File {} does not exist", file_path.display());
    }
}

fn remove_all_csv(base_dir: &PathBuf) {
    fs::read_dir(base_dir.join("sentences"))
        .unwrap()
        .for_each(|entry| {
            let entry = entry.unwrap();
            remove_file_if_exists(&entry.path());
        });

    remove_file_if_exists(&base_dir.join("tmp.csv"));
}

fn write_sentence(row: &Row, file_path: &PathBuf, headers: bool) {
    let file = fs::OpenOptions::new()
        .write(true)
        .append(true)
        .create(true)
        .open(file_path)
        .unwrap();

    let mut wtr = csv::Writer::from_writer(file);

    if headers {
        wtr.serialize(row).unwrap();
    } else {
        wtr.write_record(&[&row.language, &row.sentence, &row.timestamp])
            .unwrap();
    }
    wtr.flush().unwrap();
}

fn get_setting_store_path() -> PathBuf {
    public_dir().unwrap().join(".settings")
}

fn submit_sentence(language: &str, text: &str) -> Result<(), String> {
    let base_dir = public_dir().unwrap();

    let row = Row {
        language: language.to_string(),
        sentence: text.to_string(),
        timestamp: Utc::now().to_rfc3339(),
    };

    let tmp_file_path = base_dir.join("tmp.csv");

    let rows = count_csv_rows(&tmp_file_path);
    write_sentence(&row, &tmp_file_path, rows == 0);

    let mut store = StoreBuilder::new(".settings".into()).build();
    let mut sentences_per_csv = 100;

    store.load().unwrap_or_else(|e| {
        log::error!("Error loading store: {}", e);
    });
    match store.get("max_sentences_per_csv") {
        Some(val) => sentences_per_csv = val.as_i64().unwrap() as usize,
        None => log::error!("Error getting max_sentences_per_csv"),
    }
    match store.get("td_osc_address") {
        Some(val) => {
            let addr = val.as_str().unwrap();
            let socket = UdpSocket::bind("last-snow.local:7001").unwrap();
            let msg = rosc::encoder::encode(&OscPacket::Message(rosc::OscMessage {
                addr: "/new_row".to_string(),
                args: vec![OscType::String(row.sentence)],
            }))
            .unwrap();

            log::info!("Sending packet to {}: {:?}", addr, msg);

            socket.send_to(&msg, addr).unwrap_or_else(|e| {
                log::error!("Error sending to socket: {}", e);
                0
            });
        }
        None => log::error!("Error getting td_osc_address"),
    };

    log::info!("{}/{} rows in tmp.csv", rows + 1, sentences_per_csv);

    if rows + 1 >= sentences_per_csv {
        let new_file_path = get_new_filename(&base_dir);
        log::info!("Moving tmp.csv to {}", new_file_path.to_str().unwrap());
        fs::rename(&tmp_file_path, &new_file_path).unwrap();
    }

    Ok(())
}

fn handle_packet(packet: OscPacket) {
    let base_dir = public_dir().unwrap();

    let mut store = StoreBuilder::new(".settings".into()).build();

    store.load().unwrap_or_else(|e| {
        log::error!("Error loading store: {}", e);
    });

    match packet {
        OscPacket::Message(msg) => {
            log::info!("Received packet: {:?}", msg);

            match (msg.addr.as_str(), msg.args.as_slice()) {
                ("/td_osc_address", [OscType::String(addr)]) => {
                    store
                        .insert("td_osc_address".to_owned(), json!(addr))
                        .unwrap_or_else(|e| {
                            log::error!("Error inserting td_osc_address: {}", e);
                        });
                }
                ("/max_characters", [OscType::Int(max_characters)]) => {
                    store
                        .insert("max_characters".to_owned(), json!(max_characters))
                        .unwrap_or_else(|e| {
                            log::error!("Error inserting max_characters: {}", e);
                        });
                    // app.emit_all("max_characters", max_characters)
                    //     .unwrap_or_else(|e| {
                    //         log::error!("Error emitting max_characters: {}", e);
                    //     });
                }
                ("/max_sentences_per_csv", [OscType::Int(max_sentences_per_csv)]) => {
                    store
                        .insert(
                            "max_sentences_per_csv".to_owned(),
                            json!(max_sentences_per_csv),
                        )
                        .unwrap_or_else(|e| {
                            log::error!("Error inserting max_sentences_per_csv: {}", e);
                        });
                }
                ("/remove_all_csv", []) => {
                    remove_all_csv(&base_dir);
                }
                ("/remove_output_csv", [OscType::String(filename)]) => {
                    remove_file_if_exists(&base_dir.join("sentences").join(filename));
                }
                ("/remove_tmp_csv", []) => {
                    remove_file_if_exists(&base_dir.join("tmp.csv"));
                }
                _ => log::warn!("Invalid OSC address: {}", msg.addr),
            }

            store.save().unwrap_or_else(|e| {
                log::error!("Error saving store: {}", e);
            });
        }
        OscPacket::Bundle(bundle) => {
            for packet in bundle.content {
                handle_packet(packet);
            }
        }
    }
}

fn setup_logger() -> Result<(), fern::InitError> {
    fern::Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "{}[{}][{}] {}",
                chrono::Local::now().format("[%Y-%m-%d][%H:%M:%S]"),
                record.level(),
                record.target(),
                message
            ))
        })
        .level(LevelFilter::Debug)
        .chain(std::io::stdout())
        .chain(fern::log_file(public_dir().unwrap().join("output.log"))?)
        .apply()?;
    Ok(())
}

fn get_ibus_engine_output() -> Result<String, String> {
    let output = Command::new("ibus")
        .arg("engine")
        .output()
        .map_err(|e| e.to_string())?;

    if output.status.success() {
        let output_str = std::str::from_utf8(&output.stdout).unwrap().to_string();
        Ok(output_str)
    } else {
        let error_str = std::str::from_utf8(&output.stderr).unwrap().to_string();
        Err(error_str)
    }
}

fn set_ibus_engine(language: Languages) -> Result<(), String> {
    let output = Command::new("ibus")
        .arg("engine")
        .arg(language.value())
        .output()
        .map_err(|e| e.to_string())?;

    if output.status.success() {
        Ok(())
    } else {
        let error_str = std::str::from_utf8(&output.stderr).unwrap().to_string();
        Err(error_str)
    }
}

fn main() -> Result<(), slint::PlatformError> {
    let ui = AppWindow::new()?;

    setup_logger().unwrap();
    let mut store = StoreBuilder::new(".settings".into()).build();

    store.load().unwrap_or_else(|e| {
        log::error!("Error loading store: {}", e);
    });

    if !store.has("max_characters") {
        store
            .insert("max_characters".to_owned(), json!(160))
            .unwrap_or_else(|e| {
                log::error!("Error inserting max_characters: {}", e);
            });
    }

    if !store.has("max_sentences_per_csv") {
        store
            .insert("max_sentences_per_csv".to_owned(), json!(100))
            .unwrap_or_else(|e| {
                log::error!("Error inserting max_sentences_per_csv: {}", e);
            });
    }

    store.save().unwrap_or_else(|e| {
        log::error!("Error saving store: {}", e);
    });

    thread::spawn(move || {
        // Bind the UDP socket to listen on port 7000
        let socket = UdpSocket::bind("127.0.0.1:7000").unwrap();
        log::info!("Listening on {}", socket.local_addr().unwrap());

        let mut buf = [0u8; rosc::decoder::MTU];

        loop {
            match socket.recv_from(&mut buf) {
                Ok((size, addr)) => {
                    log::info!("Received packet with size {} from: {}", size, addr);
                    let (_, msg) = rosc::decoder::decode_udp(&buf[..size]).unwrap();
                    handle_packet(msg);
                }
                Err(e) => {
                    log::info!("Error receiving from socket: {}", e);
                    break;
                }
            }
        }
    });

    let ui_handle = ui.as_weak();
    let model = Rc::new(VecModel::from(
        Languages::iter()
            .map(|l| l.to_string().into())
            .collect::<Vec<_>>(),
    ));
    ui.set_languages(ModelRc::from(model.clone()));

    let language = get_ibus_engine_output().unwrap_or_else(|e| {
        log::error!("Error getting ibus engine output: {}", e);
        Languages::EN.value().to_string()
    });

    ui.set_selected_language(
        Languages::from_value(language.as_str())
            .unwrap()
            .to_string()
            .into(),
    );

    ui.on_select_language(move |language| {
        let language = Languages::from_str(language.as_str()).unwrap();
        set_ibus_engine(language.into()).unwrap_or_else(|e| {
            log::error!("Error setting ibus engine: {}", e);

            let language = get_ibus_engine_output().unwrap_or_else(|e| {
                log::error!("Error getting ibus engine output: {}", e);
                Languages::EN.value().to_string()
            });

            ui_handle.unwrap().set_selected_language(
                Languages::from_value(language.as_str())
                    .unwrap()
                    .to_string()
                    .into(),
            );
        });
    });

    ui.on_submit_text(|text| {
        log::info!("Received text: {}", text);
        submit_sentence("en", text.as_str()).unwrap();
    });

    ui.run()
}
