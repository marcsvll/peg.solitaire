use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use anyhow::Result;
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::TcpListener,
    sync::broadcast,
};

#[derive(Clone, Debug)]
enum ServerMessage {
    ChatMessage(String),
    BoardUpdate(String),
}

#[derive(Clone)]
struct User {
    name: String,
    _id: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let listener = TcpListener::bind("localhost:8080").await?;
    let (tx, _rx) = broadcast::channel::<ServerMessage>(10);
    let user_map: Arc<Mutex<HashMap<String, User>>> = Arc::new(Mutex::new(HashMap::new()));

    println!("Server running on localhost:8080");

    loop {
        let (mut socket, addr) = listener.accept().await?;
        let tx = tx.clone();
        let mut rx = tx.subscribe();
        let user_map = user_map.clone();

        tokio::spawn(async move {
            let (reader, mut writer) = socket.split();
            let mut reader = BufReader::new(reader);
            let mut line = String::new();

            // Ler a primeira linha para o nome de usuário
            if reader.read_line(&mut line).await? == 0 {
                return Ok::<(), anyhow::Error>(());
            }

            let username = line.trim().to_string(); // Extrai o nome de usuário
            let user = User { name: username.clone(), _id: addr.to_string() };
            user_map.lock().unwrap().insert(addr.to_string(), user.clone());
            println!("{} connected", username);
            line.clear(); // Limpa a linha para as próximas leituras

            loop {
                tokio::select! {
                    result = reader.read_line(&mut line) => {
                        let line = line.trim();
                        if result.unwrap() == 0 || line.is_empty() {
                            println!("{} disconnected", username);
                            break;
                        }

                        if line.starts_with("move") {
                            let board_update = line.to_string(); // Simplificação para o exemplo
                            tx.send(ServerMessage::BoardUpdate(board_update)).unwrap();
                        } else {
                            let chat_message = format!("{}: {}", user.name, line);
                            tx.send(ServerMessage::ChatMessage(chat_message)).unwrap();
                        }
                    },
                    result = rx.recv() => {
                        match result.unwrap() {
                            ServerMessage::ChatMessage(msg) => {
                                if let Err(e) = writer.write_all(msg.as_bytes()).await {
                                    eprintln!("Error sending message to {}: {}", username, e);
                                    break;
                                }
                            },
                            ServerMessage::BoardUpdate(board_state) => {
                                if let Err(e) = writer.write_all(board_state.as_bytes()).await {
                                    eprintln!("Error sending board update to {}: {}", username, e);
                                    break;
                                }
                            },
                        }
                    },
                }
            }

            // Remove o usuário do mapa de usuários quando ele se desconecta
            user_map.lock().unwrap().remove(&addr.to_string());
            Ok(())
        });
    }
}
