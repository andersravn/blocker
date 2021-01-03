use std::error::Error;
use std::fmt;
use std::fs::{create_dir_all, File};
use std::io::{Read, Write};
use std::str::from_utf8;
extern crate chrono;
extern crate timer;
use std::path::Path;
use std::sync::mpsc::channel;
extern crate directories;
use directories::ProjectDirs;

// I want to be able to add a website to a deny list
// When starting the blocker, it should copy the deny list into etc/hosts
// When the blocker stops, it should remove the deny list from etc/hosts
// The blocker should be started with a timer in minutes

const HOSTS_PATH: &str = "/etc/";
const HOSTS_FILE: &str = "hosts";
const FILE: &str = "urls";

pub struct Config {
    pub query: String,
    pub name: String,
    pub config_path: String,
}

impl Config {
    pub fn new(args: &[String]) -> Result<Config, &'static str> {
        if args.len() < 3 {
            return Err("not enough arguments");
        }
        let query = args[1].clone();
        let name = args[2].clone();

        if let Some(proj_dirs) = ProjectDirs::from("com", "andersravn", "blocker") {
            Ok(Config {
                query,
                name,
                config_path: format!("{}", proj_dirs.config_dir().display()),
            })
        } else {
            Ok(Config {
                query,
                name,
                config_path: String::from(""),
            })
        }
    }
}

impl fmt::Display for Config {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "(query: {}, name: {})", self.query, self.name)
    }
}

fn create_file_or_path_if_does_not_exist(path: &str, file: &str) {
    if !Path::new(path).exists() {
        create_dir_all(path).expect("Could not create path");
    }

    let full = format!("{}/{}", path, file);
    if !Path::new(&full).exists() {
        File::create(full).expect("Could not create file");
    }
}

fn read_file(path: &str, file: &str) -> Result<String, Box<dyn Error>> {
    create_file_or_path_if_does_not_exist(path, file);
    let mut file = File::open(format!("{}/{}", path, file))?;
    let stat = file.metadata()?;

    let mut buffer = vec![0; stat.len() as usize];
    file.read(&mut buffer)?;
    let value = from_utf8(&buffer)?.to_string();

    Ok(value)
}

fn write_file(path: &str, file: &str, data: &str) -> Result<(), Box<dyn Error>> {
    create_file_or_path_if_does_not_exist(path, file);
    File::create(format!("{}/{}", path, file))?.write_all(data.as_bytes())?;
    Ok(())
}

fn add_block(config: Config) {
    let localhost = "127.0.0.1";
    let contents =
        read_file(&config.config_path, FILE).expect("Something went wrong reading the file");
    let new_contents = if contents.len() == 0 {
        format!("{}\t{}", localhost, config.name)
    } else {
        format!("{}\n{}\t{}", contents, localhost, config.name)
    };

    write_file(&config.config_path, FILE, &new_contents).unwrap();
}

fn remove_lines(text: &str, start_line: &str, end_line: &str) -> String {
    let mut end_line_number = 0;
    let mut has_found_start_line = false;
    let mut has_found_end_line = false;
    let mut result = String::from("");
    for (i, line) in text.lines().enumerate() {
        if line.trim() == start_line.trim() {
            has_found_start_line = true;
        }
        if line.trim() == end_line.trim() {
            end_line_number = i;
            has_found_end_line = true;
        }

        if !has_found_start_line || (has_found_end_line && i != end_line_number) {
            if line.len() == 0 {
                result.push_str("\n");
            } else {
                result.push_str(line);
                result.push_str("\n");
            }
        }
    }

    String::from(result.replace("\n\n\n", "\n\n").trim_end())
}

fn start(config: Config) {
    let hosts = read_file(HOSTS_PATH, HOSTS_FILE).expect("Something went wrong reading hosts file");
    let mut new_hosts = remove_lines(&hosts, "# start block", "# end block");
    let urls =
        read_file(&config.config_path, FILE).expect("Something went wrong reading urls file");
    new_hosts = format!("{}\n\n# start block\n{}\n#end block", new_hosts, urls);
    write_file(HOSTS_PATH, HOSTS_FILE, &new_hosts).unwrap();

    let (tx, rx) = channel();
    let minutes = config.name.parse::<i64>().unwrap();
    let timer = timer::Timer::new();
    let _guard = timer.schedule_with_delay(chrono::Duration::minutes(minutes), move || {
        println!("Stopping...");
        let hosts =
            read_file(HOSTS_PATH, HOSTS_FILE).expect("Something went wrong reading hosts file");
        let new_hosts = remove_lines(&hosts, "# start block", "# end block");
        write_file(HOSTS_PATH, HOSTS_FILE, &new_hosts).unwrap();
        let _ignored = tx.send(());
    });

    match rx.recv() {
        Err(err) => eprintln!("Error: {}", err),
        Ok(value) => value,
    };
}

fn stop() {
    println!("Stopping...");
    let hosts = read_file(HOSTS_PATH, HOSTS_FILE).expect("Something went wrong reading hosts file");
    let new_hosts = remove_lines(&hosts, "# start block", "# end block");
    write_file(HOSTS_PATH, HOSTS_FILE, &new_hosts).unwrap();
}

pub fn run(config: Config) -> Result<(), Box<dyn Error>> {
    match config.query.as_str() {
        "add" => add_block(config),
        "start" => start(config),
        "stop" => stop(),
        _ => println!("Unknown command"),
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_result() {
        let hosts = "\
127.0.0.1	localhost
127.0.1.1	anders-ThinkPad-X240

# start block
127.0.0.1   www.facebook.com
127.0.0.1   www.twitter.com
# end block

# The following lines are desirable for IPv6 capable hosts
::1     ip6-localhost ip6-loopback";

        assert_eq!(
            String::from(
                "\
127.0.0.1	localhost
127.0.1.1	anders-ThinkPad-X240

# The following lines are desirable for IPv6 capable hosts
::1     ip6-localhost ip6-loopback"
            ),
            remove_lines(hosts, "# start block\n", "# end block\n")
        );
    }
}
