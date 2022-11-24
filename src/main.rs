use clap::{arg, command, Parser};
use std::collections::HashMap;
use std::net::TcpStream;
use std::io::{Read, Write};
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncReadExt,AsyncWriteExt};
use tokio::net::TcpListener;
use rand::Rng;

#[derive(Clone, Debug, Eq, PartialEq)]
enum TargetStatus {
    Unknown,
    Alive,
    Dead
}

#[derive(Parser, Debug)]
#[command(author="Alfred Dobradi <alfreddobradi@gmail.com>", version="0.0.1", about=None, long_about=None)]
struct Args {
    #[arg(
        short = 'b',
        long = "bind",
        env = "CRUSTY_BIND_ADDRESS",
        default_value = "127.0.0.1"
    )]
    bind_address: String,

    #[arg(
        short = 'p',
        long = "port",
        env = "CRUSTY_BIND_PORT",
        default_value = "9654"
    )]
    bind_port: String,

    #[arg(short=None, long="targets", env="CRUSTY_TARGETS", required=true)]
    targets: String,
}

#[derive(Clone, Debug)]
struct TargetMap {
    targets: HashMap<String, TargetStatus>
}

impl TargetMap {
    fn from(src: String) -> Self {
        let t: Vec<&str> = src.split(",").collect();

        let mut target_map: HashMap<String, TargetStatus> = HashMap::new();
        for target in t {
            target_map.insert(String::from(target.trim()), TargetStatus::Unknown);
        }

        TargetMap{
            targets: target_map
        }
    }

    fn pick(&self) -> Result<String, String> {
        let mut live_targets = self.targets.clone();
        live_targets.retain(|_, v| *v == TargetStatus::Alive);

        let len = live_targets.len();
        if len == 0 {
            return Err(String::from("No live targets"));
        }
        let mut rng = rand::thread_rng();
        let n = rng.gen_range(0..len);

        match live_targets.keys().nth(n) {
            Some(str) => Ok(str.to_string()),
            _ => Err(String::from("Something went wrong"))
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    // let mut clientMap: HashMap< = HashMap::new()

    println!("Bind address: {:?}", args.bind_address);
    println!("Bind port: {:?}", args.bind_port);

    let targets = Arc::new(Mutex::new(TargetMap::from(args.targets)));
    
    let tt = targets.clone();
    tokio::spawn(async move {
        loop {
            match check(&tt).await {
                Ok(_) => {
                    println!("All checks successful");
                },
                Err(failed_targets) => {
                    println!("These targets have failed their checks:");
                    for target in failed_targets {
                        println!("\t{}", target);
                    }
                },
            }

            std::thread::sleep(std::time::Duration::from_millis(30000));
        }
    });

    let listener = TcpListener::bind("127.0.0.1:8080").await?;
    loop {
        let (mut socket, address) = listener.accept().await?;

        let targets_pick = targets.clone();
        tokio::spawn(async move {
            let target = match targets_pick.lock().unwrap().pick() {
                Ok(tt) => tt,
                Err(error) => {
                    eprintln!("error picking target: {}", error);
                    return;
                }
            };
            println!("Target: {}", target);
            println!("Source: {}", address.to_string());

            let mut buf = [0; 1024];

            // In a loop, read data from the socket and write the data back.
            loop {
                let n = match socket.read(&mut buf).await {
                    // socket closed
                    Ok(n) if n == 0 => return,
                    Ok(n) => n,
                    Err(e) => {
                        eprintln!("failed to read from socket; err = {:?}", e);
                        return;
                    }
                };

                let mut b: [u8; 1024] = [0; 1024];
                let stream = TcpStream::connect(&target);
                match stream {
                    Ok(mut str) => {
                        let res = str.write(&buf[0..n]);
                        match res {
                            Ok(_) => {
                                if let Err(read_error) = str.read(&mut b[0..n]) {
                                    eprintln!("failed to read destination reply: {:?}", read_error);
                                    return;
                                }
                            }
                            Err(e) => {
                                eprintln!("failed to write data: {:?}", e);
                                return;
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("failed to connect to destination: {:?}", e);
                        return;
                    }
                }

                // Write the data back
                if let Err(e) = socket.write_all(&b[0..n]).await {
                    eprintln!("failed to write to socket; err = {:?}", e);
                    return;
                }
            }
        });
    }
}

async fn check(targets: &Arc<Mutex<TargetMap>>) -> Result<(), Vec<String>> {
    let mut failed: Vec<String> = Vec::new();
    let mut targets = targets.lock().unwrap();
    for (target, status) in targets.targets.iter_mut() {
        let stream = TcpStream::connect(target);
        let new_status;
        match stream {
            Ok(str) => {
                if let Err(_) = str.shutdown(std::net::Shutdown::Both) {
                    failed.push(target.to_string());
                    new_status = TargetStatus::Dead;
                } else {
                    new_status = TargetStatus::Alive;
                }
            }
            Err(_) => {
                failed.push(target.to_string());
                new_status = TargetStatus::Dead;
            }
        }
        *status = new_status;
    }

    if failed.len() > 0 {
        return Err(failed);
    }
    Ok(())
}