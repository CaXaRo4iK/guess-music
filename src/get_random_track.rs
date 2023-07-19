use rand::seq::SliceRandom;
use serde_json::Value;
use std::io::{Error, ErrorKind, Result};
use serde_xml_rs::from_str;
use serde::{Deserialize, Serialize};
use md5::compute;

#[derive(Debug, Clone)]
pub struct RandomTrack {
    pub title: String,
    pub version: Option<String>,
    pub data: Vec<u8>,
    pub link: String,
    pub artists: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, )]
pub struct XmlDownload {
    pub host: String,
    pub path: String,
    pub ts: String,
    pub region: String,
    pub s: String,
}

fn get_http(url: &str, auth: &str) -> Result<Value> {
    ureq::get(url)
        .set("Authorization", auth)
        .set("User-Agent", "Yandex-Music-API")
        .set("X-Yandex-Music-Client", "YandexMusicAndroid/24023231")
        .call().unwrap().into_json()
}

fn post_http(url: &str, body: &[(&str, &str)], auth: &str) -> Result<Value> {
    ureq::post(url)
        .set("Authorization", auth)
        .set("User-Agent", "Yandex-Music-API")
        .set("X-Yandex-Music-Client", "YandexMusicAndroid/24023231")
        .send_form(body).unwrap().into_json()
}

pub fn get_random_track(token: &str) -> Result<RandomTrack> {
    let base_url = "https://api.music.yandex.net";
    let sign_salt = "XGRlBW9FXlekgbPrRHuSiA";
    let auth = format!("OAuth {}", token);
    let uid = get_http(&format!("{}/account/status", base_url), &auth)?["result"]["account"]["uid"].to_string();
    let mut counter = 0;
    loop {
        if counter > 10 {
            break Err(Error::new(
                ErrorKind::NotFound,
                "Unable to get random track".to_string(),
            ));
        }
        let track_info_short = get_http(&format!("{}/users/{}/likes/tracks?if-modified-since-revision=1", base_url, uid), &auth)?["result"]["library"]["tracks"].as_array().unwrap().choose(&mut rand::thread_rng()).unwrap().to_owned();
        let track_id: String;
        if track_info_short["albumId"].is_null() {
            track_id = track_info_short["id"].as_str().unwrap().to_string();
        } else {
            track_id = format!("{}:{}", track_info_short["id"].as_str().unwrap().to_string(), track_info_short["albumId"].as_str().unwrap().to_string());
        }
        let track_info = post_http("https://api.music.yandex.net/tracks", &[("track-ids", &track_id)], &auth)?["result"][0].to_owned();
        if !track_info["shortDescription"].is_null() {
            counter += 1;
            continue;
        }
        let download_url = get_http(&format!("{}/tracks/{}/download-info", base_url, track_id), &auth)?["result"][0]["downloadInfoUrl"].as_str().unwrap().to_string();
        let data_xml = from_str::<XmlDownload>(ureq::get(download_url.as_str())
            .set("Authorization", &auth)
            .set("User-Agent", "Yandex-Music-API")
            .set("X-Yandex-Music-Client", "YandexMusicAndroid/24023231")
            .call().unwrap().into_string().unwrap().as_str()).unwrap();
        let url = format!("https://{host}/get-mp3/{hash:?}/{ts}{path}", host = data_xml.host, hash = compute((sign_salt.to_owned() + &data_xml.path[1..] + &data_xml.s).as_bytes()), ts = data_xml.ts, path = data_xml.path);
        let mut reader = ureq::get(url.as_str())
            .set("Authorization", &auth)
            .set("User-Agent", "Yandex-Music-API")
            .set("X-Yandex-Music-Client", "YandexMusicAndroid/24023231")
            .call().unwrap().into_reader();
        let mut buf = Vec::new();
        reader.read_to_end(&mut buf).unwrap();
        break Ok(RandomTrack {
            title: track_info["title"].as_str().unwrap().to_string(),
            version: (|track_info: &Value| -> Option<String> { if track_info["version"].is_null() { None } else { Some(track_info["version"].as_str().unwrap().to_string()) } })(&track_info),
            data: buf,
            link: (|| {
                if track_info_short["albumId"].is_null() {
                    format!("https://music.yandex.ru/track/{}", track_info_short["id"].as_str().unwrap().to_string())
                } else {
                    format!("https://music.yandex.ru/album/{}/track/{}", track_info_short["albumId"].as_str().unwrap().to_string(), track_info_short["id"].as_str().unwrap().to_string())
                }
            })(),
            artists: (|i: Vec<Value>| -> Vec<String>{ i.into_iter().map(|x| x["name"].as_str().unwrap().to_string()).collect() })(track_info["artists"].as_array().unwrap().to_owned()),
        });
    }
}

pub fn check_auth(token: &str) -> Result<()> {
    let base_url = "https://api.music.yandex.net";
    let auth = format!("OAuth {}", token);
    match ureq::get(&format!("{}/account/status", base_url))
        .set("Authorization", &auth)
        .set("User-Agent", "Yandex-Music-API")
        .set("X-Yandex-Music-Client", "YandexMusicAndroid/24023231")
        .call() {
        Ok(_) => Ok(()),
        Err(_) => Err(Error::new(ErrorKind::InvalidInput, "Invalid token".to_string())),
    }
}