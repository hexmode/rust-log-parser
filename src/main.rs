#![feature(unix_sigpipe)]
#[macro_use]
extern crate serde;
extern crate regex;
extern crate serde_json;
extern crate structopt;

extern crate itertools;

use std::collections::BTreeMap;
use std::io::Error;
use std::path::PathBuf;
use structopt::StructOpt;

use regex::Regex;
use std::fs::File;
use std::io::{self, BufRead, BufReader, Result as StdIOResult};

mod config;

use config::Config;

#[derive(StructOpt, Debug)]
#[structopt(name = "basic")]
struct Opt {
    //print the line which cannot be parsed and exit, default false
    #[structopt(short = "s", long = "stop")]
    stop: bool,

    //read from file, default is stdin
    #[structopt(short = "i", long = "input-file", parse(from_os_str))]
    file: Option<PathBuf>,

    //use config file
    #[structopt(short = "c", long = "config", parse(from_os_str))]
    config: Option<PathBuf>,

    #[structopt(short = "f", long = "format", default_value = "json")]
    format: String,
}

const JSON: &str = "json";

fn get_buffer(name: &Option<PathBuf>) -> Box<dyn BufRead> {
    match name {
        None => Box::new(BufReader::new(io::stdin())),
        Some(file) => Box::new(BufReader::new(File::open(file).unwrap())),
    }
}

fn get_config(config: &Option<PathBuf>) -> config::Config {
    match config {
        None => config::default(),
        Some(f) => match config::from_file(f.to_path_buf()) {
            Ok(config) => config,
            Err(v) => {
                eprintln!("{}", v);
                std::process::exit(-1);
            }
        },
    }
}

fn get_format(format: &String, matches: &BTreeMap<String, String>) -> String {
    if *format != JSON {
        let mut found_format = false;
        for val in matches.values() {
            if val.eq(format) {
                found_format = true
            }
        }
        if !found_format {
            let a: Vec<_> = matches.values().collect();
            eprintln!(
                "Provided format '{}' does not exit, allowed values are {}",
                format,
                itertools::join(a, ", ")
            );
            std::process::exit(-1);
        }
    }
    format.to_string()
}

fn handle_line(line: Result<String, Error>, opt: &Opt) {
    let config = get_config(&opt.config);
    let format = get_format(&opt.format, &config.matches);

    match line {
        Err(_) => {
            println!("Failed to read line");
            std::process::exit(-1);
        }
        Ok(l) => {
            if let Some(entry) = parse(l.clone(), &config) {
                if format == JSON {
                    let json = serde_json::to_string(&entry).unwrap();
                    println!("{}", json);
                } else {
                    println!("{}", entry.get(&format).unwrap());
                }
            } else if opt.stop {
                eprintln!("parse failed: {:?}", l);
                std::process::exit(-1);
            }
        }
    }
}

fn parse(l: String, config: &Config) -> Option<BTreeMap<String, String>> {
    let mut dummy: BTreeMap<String, String> = BTreeMap::new();
    let re = Regex::new(&config.regex).unwrap();
    let parsed_value = re.captures(&l);

    parsed_value.as_ref()?;
    let caps = parsed_value.unwrap();

    for (k, v) in config.matches.iter() {
        dummy.insert(
            v.to_string(),
            caps.get(k.parse().unwrap())
                .map_or("", |m| m.as_str())
                .to_string(),
        );
    }
    Some(dummy)
}

#[unix_sigpipe = "sig_dfl"]
fn main() -> StdIOResult<()> {
    let opt = Opt::from_args();
    let buffer: Box<dyn BufRead> = get_buffer(&opt.file);

    for line in buffer.lines() {
        handle_line(line, &opt);
    }
    Ok(())
}
