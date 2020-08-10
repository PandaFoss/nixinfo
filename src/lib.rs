use glob::glob;
use std::env;
use std::fs::{metadata, read_to_string, File};
use std::io::{BufReader, Read};
use std::process::{Command, Output};

// Shared functions for the lib
fn read(file: File) -> Result<String, Box<dyn std::error::Error>> {
    let mut buf_reader = BufReader::new(file);
    let mut contents = String::new();
    buf_reader.read_to_string(&mut contents)?;
    Ok(contents)
}

fn line(file: File, line: usize) -> String {
    let contents = read(file).unwrap();
    let file_vector: Vec<&str> = contents.split('\n').collect();
    file_vector[line].to_string()
}

// CPU info
fn get(file: File, x: usize) -> String {
    let line = line(file, x);
    let line_vec: Vec<&str> = line.split(':').collect();
    line_vec[1].to_string()
}

fn format(info: String) -> String {
    info.replace("(TM)", "")
        .replace("(R)", "")
        .replace("     ", " ")
}

/// Obtain the temp of the CPU, only tested on rpi, outputs to a string
pub fn temp() -> String {
    if metadata("/sys/class/thermal/thermal_zone0/temp").is_ok() {
        let file = File::open("/sys/class/thermal/thermal_zone0/temp").unwrap();
        let raw_temp = read(file).unwrap().trim().parse::<i64>().unwrap();
        format!("{}", raw_temp / 1000)
    } else {
        "N/A (could not read /sys/class/thermal/thermal_zone0/temp)".to_string()
    }
}

/// Obtain CPU model, outputs to a string
pub fn cpu() -> String {
    if metadata("/proc/cpuinfo").is_ok() {
        let file = File::open("/proc/cpuinfo").unwrap();
        if metadata("/sys/firmware/devicetree/base/model").is_ok() {
            if read_to_string("/sys/firmware/devicetree/base/model")
                .unwrap()
                .starts_with("Raspberry")
            {
                let info = get(file, 1); // Line 2
                format(info)
            } else {
                let info = get(file, 4); // Line 5
                format(info)
            }
        } else {
            let info = get(file, 4); // Line 5
            format(info)
        }
    } else {
        "N/A (could not read /proc/cpuinfo)".to_string()
    }
}

// Device info
/// Obtain name of device, outputs to a string
pub fn device() -> String {
    if metadata("/sys/devices/virtual/dmi/id/product_name").is_ok() {
        read_to_string("/sys/devices/virtual/dmi/id/product_name").unwrap()
    } else if metadata("/sys/firmware/devicetree/base/model").is_ok() {
        read_to_string("/sys/firmware/devicetree/base/model").unwrap()
    } else {
        "N/A (could not obtain name of device)".to_string()
    }
}

// Distro info
fn dist(path: &str) -> String {
    let file = File::open(path).unwrap();
    let line: String = line(file, 0); // Expects NAME= to be on first line
    let distro_vec: Vec<&str> = line.split('=').collect();
    String::from(distro_vec[1])
}

/// Obtain the distro name, outputs to a string
pub fn distro() -> String {
    if metadata("/bedrock/etc/os-release").is_ok() {
        dist("/bedrock/etc/os-release")
    } else if metadata("/etc/os-release").is_ok() {
        dist("/etc/os-release")
    } else if metadata("/usr/lib/os-release").is_ok() {
        dist("/usr/lib/os-release")
    } else {
        "N/A (could not obtain distro name, please file a bug as your os-release file may just be in a weird place)".to_string()
    }
}

// Environment info
fn de() -> String {
    env::var("XDG_DESKTOP_SESSION")
        .or_else(|_| env::var("XDG_CURRENT_DESKTOP"))
        .or_else(|_| env::var("DESKTOP_SESSION"))
        .unwrap_or_else(|_| "N/A".to_string())
}

fn wm() -> String {
    let path = format!("{}/.xinitrc", env::var("HOME").unwrap());
    if std::fs::metadata(&path).is_ok() {
        let file = File::open(&path).unwrap();
        let contents = read(file).unwrap();
        let line = contents.lines().last().unwrap();
        line.split(' ').last().unwrap().to_string()
    } else {
        "N/A (could not open $HOME/.xinitrc)".to_string()
    }
}

/// Obtains the name of the user's DE or WM, outputs to a string
pub fn environment() -> String {
    let de = de();
    if de == "N/A" {
        wm()
    } else {
        de
    }
}

// Env variables
/// Obtain the contents of an env variable, outputs to a string
pub fn env(var: String) -> String {
    // $SHELL and $USER are set automatically, the only env variable it would fail on is $EDITOR
    env::var(var).expect("Could not read $EDITOR, are you sure it's set?")
}

// Hostname
/// Obtain the hostname, outputs to a string
pub fn hostname() -> String {
    if metadata("/etc/hostname").is_ok() {
        read_to_string("/etc/hostname").unwrap()
    } else {
        "N/A (could not read /etc/hostname)".to_string()
    }
}

// Kernel version
/// Obtain the kernel version, outputs to a string
pub fn kernel() -> String {
    if metadata("/proc/sys/kernel/osrelease").is_ok() {
        read_to_string("/proc/sys/kernel/osrelease").unwrap()
    } else {
        "N/A (could not obtain kernel version)".to_string()
    }
}

// Memory info
/// Obtain total memory in MBs, outputs to a string
pub fn memory() -> String {
    if metadata("/proc/meminfo").is_ok() {
        let file = File::open("/proc/meminfo").unwrap();
        let total_line = line(file, 0); // MemTotal should be on the first line
        let total_vec: Vec<&str> = total_line.split(':').collect();
        let total = total_vec[1].replace("kB", "");
        let total = total.trim().parse::<i64>().unwrap() / 1024;
        total.to_string() + " MB"
    } else {
        "N/A (could not read /proc/meminfo)".to_string()
    }
}

// Music info
/// Connects to mpd, and obtains music info in the format "artist - album (date) - title", outputs to a string
#[cfg(feature = "music")]
pub fn music() -> String {
    let mut c = mpd::Client::connect("127.0.0.1:6600").unwrap();
    let song: mpd::Song = c.currentsong().unwrap().unwrap();
    let na = "N/A".to_string();
    let tit = song.title.as_ref().unwrap();
    let art = song.tags.get("Artist").unwrap_or(&na);
    let alb = song.tags.get("Album").unwrap_or(&na);
    let dat = song.tags.get("Date").unwrap_or(&na);
    format!("{} - {} ({}) - {}", art, alb, dat, tit)
}

#[cfg(feature = "nomusic")]
pub fn music() -> String {
    "N/A (music feature must be used to pull in the mpd dependency)".to_string()
}

// Package info
fn count(output: Output) -> usize {
    let raw_list = String::from_utf8_lossy(&output.stdout);
    let list: Vec<&str> = raw_list.split('\n').collect();
    list.iter().count() - 1 // -1 to deal with newline at end of output
}

/// Obtain list of packages based on what manager is given as an arg, outputs to a string
pub fn packages(manager: &str) -> String {
    match manager {
        "apk" => {
            let output = Command::new("apk")
                .arg("info")
                .output()
                .expect("Could not run apk.");
            format!("{}", count(output))
        }
        "apt" => {
            let output = Command::new("apt")
                .args(&["list", "--installed"])
                .output()
                .expect("Could not run apt.");
            format!("{}", count(output) - 1) // -1 to deal with "Listing..."
        }
        "dnf" => {
            let output = Command::new("dnf")
                .args(&["list", "installed"])
                .output()
                .expect("Could not run dnf.");
            format!("{}", count(output))
        }
        "dpkg" => {
            let output = Command::new("dpkg-query")
                .args(&["-f", "'${binary:Package}\n'", "-W"])
                .output()
                .expect("Could not run dpkg-query.");
            format!("{}", count(output))
        }
        "eopkg" => {
            let output = Command::new("eopkg")
                .arg("list-installed")
                .output()
                .expect("Could not run eopkg.");
            format!("{}", count(output))
        }
        "pacman" => {
            let output = Command::new("pacman")
                .args(&["-Q", "-q"])
                .output()
                .expect("Could not run pacman.");
            format!("{}", count(output))
        }
        "pip" => {
            let output = Command::new("pip")
                .arg("list")
                .output()
                .expect("Could not run pip.");
            format!("{}", count(output) - 2) // -2 to deal with header lines in output
        }
        "portage" => {
            let content = read(File::open("/var/lib/portage/world").unwrap()).unwrap();
            let file_vector: Vec<&str> = content.split('\n').collect();

            let mut list: Vec<String> = Vec::new();
            for entry in glob("/var/db/pkg/*/*/").expect("Failed to read glob pattern") {
                match entry {
                    Ok(path) => list.push(path.display().to_string()),
                    Err(e) => println!("{:?}", e),
                }
            }

            format!(
                "{} (explicit), {} (total)",
                file_vector.iter().count() - 1,
                list.iter().count()
            )
        }
        "rpm" => {
            let output = Command::new("rpm")
                .args(&["-q", "-a"])
                .output()
                .expect("Could not run rpm.");
            format!("{}", count(output))
        }
        "xbps" => {
            let output = Command::new("xbps-query")
                .arg("list-installed")
                .output()
                .expect("Could not run xbps-query.");
            format!("{}", count(output))
        }
        _ => format!(
            "N/A ({} is not supported, please file a bug to get it added!)",
            manager
        ),
    }
}

// Terminal info
fn ppid(file: File) -> String {
    let ppid = line(file, 6);
    ppid.split(':').collect::<Vec<&str>>()[1].to_string()
}

fn name(ppid: String) -> String {
    let path = format!("/proc/{}/status", ppid.trim());
    let file = File::open(path).unwrap();
    let line = line(file, 0);
    line.split(':').collect::<Vec<&str>>()[1].to_string()
}

fn info(process_name: String, process_id: String) -> String {
    if process_name.ends_with("sh")
        || process_name == "ion"
        || process_name == "screen"
        || process_name == "tmux"
        || process_name.starts_with("tmux")
    {
        let path = format!("/proc/{}/status", process_id);
        let new_ppid = ppid(File::open(path).unwrap()).trim().replace("\n", "");
        let new_name = name(new_ppid.clone()).trim().replace("\n", "");
        if new_name.ends_with("sh")
            || new_name == "ion"
            || new_name == "screen"
            || new_name == "tmux"
            || new_name.starts_with("tmux")
        {
            let path = format!("/proc/{}/status", new_ppid);
            let new_ppid = ppid(File::open(path).unwrap()).trim().replace("\n", "");
            name(new_ppid).trim().replace("\n", "")
        } else {
            new_name.trim().replace("\n", "")
        }
    } else {
        process_name.trim().replace("\n", "")
    }
}

/// Obtain the name of the terminal being used, outputs to a string
pub fn terminal() -> String {
    let id = std::process::id();
    let path = format!("/proc/{}/status", id);
    if metadata(path.clone()).is_ok() {
        let process_id = ppid(File::open(path).unwrap()).trim().replace("\n", "");
        let process_name = name(process_id.clone()).trim().replace("\n", "");
        let info = info(process_name, process_id);
        if info == "systemd" || info == "" {
            "N/A (could not determine the terminal, this could be an issue of using tmux)"
                .to_string()
        } else {
            info
        }
    } else {
        format!("N/A (could not read {})", path)
    }
}

// Uptime
fn duration(uptime: i64) -> (String, String, String) {
    let days = if uptime > 86400 {
        let days_pre = uptime / 60 / 60 / 24;
        days_pre.to_string() + "d"
    } else {
        "".to_string()
    };
    let hours = if uptime > 3600 {
        let hours_pre = (uptime / 60 / 60) % 24;
        hours_pre.to_string() + "h"
    } else {
        "".to_string()
    };
    let minutes = if uptime > 60 {
        let minutes_pre = (uptime / 60) % 60;
        minutes_pre.to_string() + "m"
    } else {
        "".to_string()
    };
    (days, hours, minutes)
}

/// Obtains the current uptime of the system, outputs to a string
pub fn uptime() -> String {
    if metadata("/proc/uptime").is_ok() {
        let raw_uptime = read_to_string("/proc/uptime").unwrap();
        let uptime_vec: Vec<&str> = raw_uptime.split('.').collect();
        let uptime = uptime_vec[0].parse::<i64>().unwrap();
        let (days, hours, minutes) = duration(uptime);
        format!("{} {} {}", days, hours, minutes).trim().to_string()
    } else {
        "N/A (could not obtain read /proc/uptime)".to_string()
    }
}