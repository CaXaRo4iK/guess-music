mod get_random_track;

use std::io::Cursor;
use std::sync::mpsc;
use rodio::{Decoder, OutputStream, source::Source};
use crate::get_random_track::{get_random_track, RandomTrack, check_auth};
use std::thread;
use std::sync::mpsc::{Sender, TryRecvError};
use std::io::Write;
use std::process::exit;
use colored::Colorize;
use sublime_fuzzy::{FuzzySearch, Scoring};
use deunicode::deunicode;

fn main() {
    let (token, mut score) = get_token_and_score();
    ctrlc::set_handler(move || {
        println!("\n{} Но ваш счёт не сохранён, в следующий раз используйте {}, чтобы выйти из игры", "Пока!".bright_blue(), "exit".bright_red());
        exit(0);
    }).expect("Error setting Ctrl-C handler");
    'main: loop {
        let (tx, rx) = mpsc::channel();
        let rand = match get_random_track(token.as_str()) {
            Ok(r) => r,
            Err(_) => {
                println!("{}", "Нельзя получить песню, проверьте свою библиотеку на наличие песен и повторите попытку".bright_red());
                exit(1);
            }
        };
        let data = rand.data.clone();
        thread::spawn(move || {
            let (_stream, stream_handle) = OutputStream::try_default().unwrap();
            let source = Decoder::new(Cursor::new(data)).unwrap();
            stream_handle.play_raw(source.convert_samples()).expect("F");
            loop {
                thread::sleep(std::time::Duration::from_millis(500));
                match rx.try_recv() {
                    Ok(_) | Err(TryRecvError::Disconnected) => {
                        break;
                    }
                    Err(TryRecvError::Empty) => {}
                }
            }
        });
        print!("{}{}", "\nВведите имя песни или имя исполнителя".bright_green(), " >> ");
        std::io::stdout().flush().unwrap();
        let mut answer = String::new();
        std::io::stdin().read_line(&mut answer).unwrap();
        answer = deunicode(answer.trim());
        let scoring = Scoring {
            bonus_consecutive: 128,
            bonus_word_start: 128,
            ..Scoring::default()
        };
        let search = FuzzySearch::new(&answer, &deunicode(&rand.title)).score_with(&scoring).case_insensitive().best_match();
        if search.is_none() || search.unwrap().score() < 1000 {
            let search = FuzzySearch::new(&answer, &deunicode(&rand.version.clone().unwrap_or(String::new()))).score_with(&scoring).case_insensitive().best_match();
            if search.is_none() || search.unwrap().score() < 1000 {
                for artist in rand.artists.clone() {
                    let search = FuzzySearch::new(&answer, &deunicode(&artist)).score_with(&scoring).case_insensitive().best_match();
                    if search.is_some() && search.unwrap().score() > 1000 {
                        score += 1;
                        if win(tx, rand, true, &score) {
                            continue 'main;
                        } else {
                            break 'main;
                        }
                    }
                }
                score -= 1;
                if win(tx, rand, false, &score) {
                    continue 'main;
                } else {
                    break 'main;
                }
            } else {
                score += 2;
                if win(tx, rand, true, &score) {
                    continue 'main;
                } else {
                    break 'main;
                }
            }
        } else {
            score += 2;
            if win(tx, rand, true, &score) {
                continue 'main;
            } else {
                break 'main;
            }
        }
    }
    println!("{}, Ваш счёт: {}", "Пока!".bright_blue(), score.to_string().magenta());
    std::fs::write(std::env::current_exe().unwrap().parent().unwrap().to_str().unwrap().to_string() + "/token", format!("{}\n{}", token, score)).unwrap();
    std::io::stdin().read_line(&mut String::new()).unwrap();
}

fn win(tx: Sender<()>, rand: RandomTrack, win: bool, score: &i64) -> bool {
    tx.send(()).expect("TODO: panic message");
    if win {
        if rand.version.is_none() {
            println!("{}\nИмя трека: {}, Исполнители: {}, Счёт: {}\nСсылка >> {}", "Ты угадал !!!".green(), rand.title.magenta(), rand.artists.join(", ").magenta(), score.to_string().magenta(), rand.link.green());
        } else {
            println!("{}\nИмя трека: {}, Версия: {}, Исполнители: {}, Счёт: {}\nСсылка >> {}", "Ты угадал !!!".green(), rand.title.magenta(), rand.version.unwrap().magenta(), rand.artists.join(", ").magenta(), score.to_string().magenta(), rand.link.green());
        }
    } else {
        if rand.version.is_none() {
            println!("{}\nИмя трека: {}, Исполнители: {}, Счёт: {}\nСсылка >> {}", "Ты не угадал :)".red(), rand.title.magenta(), rand.artists.join(", ").magenta(), score.to_string().magenta(), rand.link.green());
        } else {
            println!("{}\nИмя трека: {}, Версия: {}, Исполнители: {}, Счёт: {}\nСсылка >> {}", "Ты не угадал :)".red(), rand.title.magenta(), rand.version.unwrap().magenta(), rand.artists.join(", ").magenta(), score.to_string().magenta(), rand.link.green());
        }
    }
    print!("{}{}", "Ещё раз?".bright_blue(), " >> ");
    let mut answer = String::new();
    std::io::stdout().flush().unwrap();
    std::io::stdin().read_line(&mut answer).unwrap();
    return if ["y", "yes", "Да", "да", "Н", "н", "YES", "Y", "t", "T", "h", "H", "р", "Р", "е", "Е", "У", "у"].contains(&answer.trim()) {
        true
    } else {
        false
    };
}

fn get_token_second(path: String) -> String {
    print!("Введите токен. Мануал - https://yandex-music.readthedocs.io/en/main/token.html >> ");
    let mut token = String::new();
    std::io::stdout().flush().unwrap();
    std::io::stdin().read_line(&mut token).unwrap();
    token = token.trim().to_string();
    match check_auth(token.as_str()) {
        Ok(_) => {}
        Err(_) => {
            token = get_token_second(path.clone());
        }
    };
    std::fs::write(path, token.clone() + "\n0").unwrap();
    token
}

fn get_token_and_score() -> (String, i64) {
    let path = std::env::current_exe().unwrap().parent().unwrap().to_str().unwrap().to_string() + "/token";
    match std::fs::read_to_string(&path) {
        Ok(token) => {
            let token_raw: Vec<&str> = token.split('\n').collect();
            let (token, score) = (|| -> (String, i64) {
                if token_raw.len() != 2 {
                    (token_raw[0].to_string(), 0i64)
                } else {
                    (token_raw[0].to_string(), token_raw[1].parse::<i64>().unwrap())
                }
            })();
            print!("Ваш счёт {}. Использовать предыдущий токен? {} >> ", score.to_string().magenta(), token.green());
            let mut answer = String::new();
            std::io::stdout().flush().unwrap();
            std::io::stdin().read_line(&mut answer).unwrap();
            return if ["y", "yes", "Да", "да", "Н", "н", "YES", "Y", "t", "T", "h", "H", "р", "Р", "е", "Е", "У", "у"].contains(&answer.trim()) {
                (token, score)
            } else {
                (get_token_second(path), 0)
            };
        }
        Err(_) => {
            (get_token_second(path), 0)
        }
    }
}


