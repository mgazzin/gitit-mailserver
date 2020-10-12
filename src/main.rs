#![allow(bare_trait_objects)]
#[macro_use]
extern crate serde_derive;
extern crate env_logger;
extern crate log;
extern crate serde;
extern crate serde_json;

use clap::{App, Arg};
use std::error::Error;
use std::fmt;
use std::fs::File;
use std::io::BufReader;
use std::io::Write;
use std::path::Path;

use serde::ser::{Serialize, SerializeStruct, Serializer};

extern crate imap;
extern crate native_tls;

use native_tls::TlsConnector;
use std::env;

extern crate mailparse;

use mailparse::*;

#[derive(Deserialize, Debug, Clone)]
struct Link {
    id: u16,
    link: String,
}

#[derive(Deserialize, Debug)]
struct Category {
    category: String,
    subcategory: String,
    links: Vec<Link>,
}

struct RstDoc {
    list: Vec<Category>,
}

impl RstDoc {
    fn new() -> Self {
        Self {
            list: Vec::<Category>::new(),
        }
    }

    fn read<P: AsRef<Path>>(&mut self, path: P) -> Result<&mut Self, Box<Error>> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);

        // Read the JSON contents of the file as a vector of instances of `Link`.
        let c: Vec<Category> = serde_json::from_reader(reader)?;
        for x in c {
            self.list.push(x);
        }
        Ok(self)
    }

    fn write<P: AsRef<Path>>(&mut self, path: P) -> Result<&mut Self, Box<Error>> {
        let file = File::create(path)?;

        serde_json::to_writer_pretty(&file, &self.list)?;
        Ok(self)
    }

    fn append(&mut self, cat: &Category) -> Result<Option<String>, Box<Error>> {
        let mut cat_found: bool = false;
        for x in self.list.iter_mut() {
            if x.category == cat.category && x.subcategory == cat.subcategory {
                cat_found = true;
                let mut link_found: bool = false;
                for xl in x.links.iter_mut() {
                    if xl.id == cat.links[0].id && xl.link == cat.links[0].link {
                        link_found = true;
                    }
                }
                if !link_found {
                    let l = Link {
                        id: cat.links[0].id,
                        link: cat.links[0].link.clone(),
                    };
                    x.links.push(l);
                }
            }
        }
        if !cat_found {
            let c = Category {
                category: cat.category.clone(),
                subcategory: cat.subcategory.clone(),
                links: vec![cat.links[0].clone()],
            };
            self.list.push(c);
        }
        Ok(None)
    }
}

impl Serialize for Category {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // 3 is the number of fields in the struct.
        let mut state = serializer.serialize_struct("Category", 3)?;
        state.serialize_field("category", &self.category)?;
        state.serialize_field("subcategory", &self.subcategory)?;
        state.serialize_field("links", &self.links)?;
        state.end()
    }
}

impl Serialize for Link {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // 3 is the number of fields in the struct.
        let mut state = serializer.serialize_struct("Link", 3)?;
        state.serialize_field("id", &self.id)?;
        state.serialize_field("link", &self.link)?;
        state.end()
    }
}

impl fmt::Display for RstDoc {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "---\n")?;
        write!(f, "write: rst+lhs\n")?;
        write!(f, "...\n")?;

        let mut old_category = String::from("");
        let mut old_subcategory = String::from("");
        for x in &self.list {
            if x.category != old_category {
                write!(f, "\n")?;
                write!(f, "{}\n", x.category)?;
                for _ in 0..x.category.chars().count() {
                    write!(f, "=")?;
                }
                write!(f, "\n\n")?;
                old_category = x.category.clone();
            }
            if x.subcategory != old_subcategory {
                write!(f, "{}\n", x.subcategory)?;
                for _ in 0..x.subcategory.chars().count() {
                    write!(f, "-")?;
                }
                write!(f, "\n\n")?;
                old_subcategory = x.subcategory.clone();
            }
            for l in &x.links {
                write!(f, "`{}`_\n", l.link)?;
            }
        }
        write!(f, "\n\n")?;
        Ok(())
    }
}

#[derive(Debug, Deserialize, Eq, PartialEq)]
enum MailCommand {
    LINK,
}

impl MailCommand {
    #[allow(dead_code)]
    fn from_str(s: &str) -> Option<MailCommand> {
        match s {
            "LINK" => Some(MailCommand::LINK),
            _ => None,
        }
    }

    #[allow(dead_code)]
    fn as_str(&self) -> &'static str {
        match self {
            MailCommand::LINK => "LINK",
        }
    }
}

#[derive(Debug, Deserialize, Eq, PartialEq)]
enum MailOperation {
    ADD,
}

impl MailOperation {
    #[allow(dead_code)]
    fn from_str(s: &str) -> Option<MailOperation> {
        match s {
            "ADD" => Some(MailOperation::ADD),
            _ => None,
        }
    }

    #[allow(dead_code)]
    fn as_str(&self) -> &'static str {
        match self {
            MailOperation::ADD => "ADD",
        }
    }
}

#[derive(Debug, Deserialize, Eq, PartialEq)]
struct CsvCommand {
    command: String,
    operation: String,
    category: String,
    subcategory: String,
    url: String,
}

#[derive(Debug, Deserialize, Eq, PartialEq)]
struct MailCommands {
    commands: Vec<CsvCommand>,
}

impl MailCommands {
    fn new() -> Self {
        Self {
            commands: Vec::<CsvCommand>::new(),
        }
    }

    fn read(
        &mut self,
        host: String,
        username: String,
        password: String,
        port: u16,
    ) -> Result<Option<String>, Box<dyn Error + 'static>> {
        let domain: &str = host.as_str();

        let tls = TlsConnector::builder().build().unwrap();

        // we pass in the domain twice to check that the server's TLS
        // certificate is valid for the domain we're connecting to.
        let client = imap::connect((domain, port), domain, &tls).unwrap();

        // the client we have here is unauthenticated.
        // to do anything useful with the e-mails, we need to log in
        let mut imap_session = client
            .login(username.as_str(), password.as_str())
            .map_err(|e| e.0)?;

        // we want to fetch the first email in the INBOX mailbox
        imap_session.select("INBOX")?;

        // fetch message number 1 in this mailbox, along with its RFC822 field.
        // RFC 822 dictates the write of the body of e-mails
        let mut i: u32 = 1;
        loop {
            let messages = imap_session.fetch(i.to_string(), "RFC822")?;
            let message = if let Some(m) = messages.iter().next() {
                m
            } else {
                break;
            };

            // extract the message's body
            let body = message.body().expect("message did not have a body!");
            let body = std::str::from_utf8(body)
                .expect("message was not valid utf-8")
                .to_string();

            let parsed = parse_mail(body.as_bytes()).unwrap();
            let sub = parsed.headers.get_first_value("Subject").unwrap();

            let mut reader = csv::ReaderBuilder::new()
                .has_headers(false)
                .delimiter(b';')
                .from_reader(sub.as_bytes());
            let mut iter = reader.deserialize();

            if let Some(result) = iter.next() {
                let record: CsvCommand = result?;
                self.commands.push(record);
            }
            i += 1;
        }

        // be nice to the server and log out
        imap_session.logout()?;

        Ok(None)
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let matches = App::new("gitit-mailserver")
        .version("1.0")
        .author("Marco Gazzin <gazzin.marco@gmail.com>")
        .about("Process gmail and add new content to gitit local instance")
        .arg(
            Arg::new("jsonfilepath")
                .short('j')
                .long("jsonfilepath")
                .value_name("JSONFILEPATH")
                .about("Sets json file")
                .required(true),
        )
        .arg(
            Arg::new("rstfilepath")
                .short('r')
                .long("rstfilepath")
                .value_name("RSTFILEPATH")
                .about("Sets rst file")
                .required(true),
        )
        .arg(
            Arg::new("v")
                .short('v')
                .multiple(true)
                .about("Sets the level of verbosity"),
        )
        .get_matches();

    std::env::set_var("LOG_LEVEL", "info");
    env_logger::init_from_env("LOG_LEVEL");

    let mut jsonfile_path = String::from("test.json");
    let mut rstfile_path = String::from("test.rst");

    if let Some(o) = matches.value_of("jsonfilepath") {
        jsonfile_path = (*o).to_string();
    }

    if let Some(o) = matches.value_of("rstfilepath") {
        rstfile_path = (*o).to_string();
    }

    log::info!(
        "Using json file: {} and RestructuredText file: {}",
        jsonfile_path,
        rstfile_path
    );

    let mut vec_link: RstDoc = RstDoc::new();
    vec_link.read(&jsonfile_path).unwrap();

    let imap_host = env::var("IMAP_HOST").expect("Missing or invalid env var: IMAP_HOST");
    let imap_username =
        env::var("IMAP_USERNAME").expect("Missing or invalid env var: IMAP_USERNAME");
    let imap_password =
        env::var("IMAP_PASSWORD").expect("Missing or invalid env var: IMAP_PASSWORD");
    let imap_port: u16 = env::var("IMAP_PORT")
        .expect("Missing or invalid env var: IMAP_PORT")
        .to_string()
        .parse()
        .unwrap();

    let r: &mut MailCommands = &mut MailCommands::new();

    if let Some(_email) = r.read(imap_host, imap_username, imap_password, imap_port)? {
        log::info!("Emails read properly.");
    }

    for a in r.commands.iter_mut() {
        let cmd = a.command.as_str();
        match cmd {
            "LINK" => {
                let op = a.operation.as_str();
                match op {
                    "ADD" => {
                        let new_link = Category {
                            category: a.category.clone(),
                            subcategory: a.subcategory.clone(),
                            links: vec![Link {
                                id: 1,
                                link: a.url.clone(),
                            }],
                        };
                        log::info!("{:?}", new_link);
                        vec_link.append(&new_link)?;
                    }
                    _ => log::warn!("Operation error {}", op),
                }
            }
            _ => log::warn!("Command error [{}]", cmd),
        }
    }

    log::info!("Writing to file {} ...", rstfile_path);
    let mut write_file = File::create(rstfile_path)?;
    write!(write_file, "{}", vec_link)?;

    log::info!("Writing JSON to file {} ...", &jsonfile_path);
    vec_link.write(jsonfile_path)?;
    Ok(())
}
