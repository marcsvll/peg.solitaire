use crossterm::event::{
    Event,
    KeyCode::{self, Char},
};
use tracing::error;
use tui_input::backend::crossterm::EventHandler;

use crate::{model::model::ActiveTab, model::model::SolitarieCell, InputMode, Message, Model};

fn parse_move_command(command: &str) -> Option<((usize, usize), (usize, usize))> {
    let parts: Vec<&str> = command.split('-').collect();
    if parts.len() == 2 {
        let from = parts[0];
        let to = parts[1];

        // Convertendo de notação alfanumérica (ex.: "A3") para índices de matriz (ex.: (0, 2))
        let from_idx = convert_to_board_index(from);
        let to_idx = convert_to_board_index(to);

        if from_idx.is_some() && to_idx.is_some() {
            return Some((from_idx.unwrap(), to_idx.unwrap()));
        }
    }
    None
}

fn convert_to_board_index(pos: &str) -> Option<(usize, usize)> {
    let bytes = pos.as_bytes();
    if bytes.len() == 2 {
        let row = (bytes[0] as usize).saturating_sub(b'A' as usize);
        let col = (bytes[1] as usize).saturating_sub(b'1' as usize);

        if row < 7 && col < 7 { // Considerando um tabuleiro 7x7
            return Some((row, col));
        }
    }
    None
}

fn is_valid_move(from_row: usize, from_col: usize, to_row: usize, to_col: usize, board: &Vec<Vec<SolitarieCell>>) -> bool {
    // Verificar se 'from' e 'to' estão dentro do tabuleiro e se 'from' tem um pino e 'to' está vazio
    if from_row >= board.len() || from_col >= board[0].len() || to_row >= board.len() || to_col >= board[0].len() {
        return false;
    }

    let from_cell = board[from_row][from_col];
    let to_cell = board[to_row][to_col];

    if from_cell != SolitarieCell::Peg || to_cell != SolitarieCell::Empty {
        return false;
    }

    // Verificar se o movimento é horizontal ou vertical e se existe um pino para ser "pulado"
    let (jump_row, jump_col) = ((from_row + to_row) / 2, (from_col + to_col) / 2);

    if (from_row == to_row && (from_col as i32 - to_col as i32).abs() == 2 && board[jump_row][jump_col] == SolitarieCell::Peg) ||
       (from_col == to_col && (from_row as i32 - to_row as i32).abs() == 2 && board[jump_row][jump_col] == SolitarieCell::Peg) {
        true
    } else {
        false
    }
}


fn make_move(from_row: usize, from_col: usize, to_row: usize, to_col: usize, board: &mut Vec<Vec<SolitarieCell>>) {
    // Mover o pino
    board[from_row][from_col] = SolitarieCell::Empty;
    board[to_row][to_col] = SolitarieCell::Peg;

    // Remover o pino que foi pulado
    let (jump_row, jump_col) = ((from_row + to_row) / 2, (from_col + to_col) / 2);
    board[jump_row][jump_col] = SolitarieCell::Empty;
}

pub fn update(model: &mut Model, message: Message) {
    match message {
        Message::Key(key) => match model.input_mode {
            InputMode::Normal => match key.code {
                Char('q') => {
                    if let Err(e) = model.message_tx.send(Message::Quit) {
                        error!("Failed to send quit message: {}", e)
                    }
                }
                KeyCode::Enter => {
                    if model.active_tab == ActiveTab::Chat || model.active_tab == ActiveTab::Solitaire || !model.is_user_registered {
                        model.input_mode = InputMode::Editing;
                    }
                }
                KeyCode::Tab => {
                    model.active_tab = match model.active_tab {
                        ActiveTab::Chat => ActiveTab::Logs,
                        ActiveTab::Logs => ActiveTab::Solitaire,
                        ActiveTab::Solitaire => ActiveTab::Chat,
                    }
                }
                _ => {}
            },
            InputMode::Editing => match key.code {
                KeyCode::Enter => {
                    if model.is_user_registered {
                        if model.active_tab == ActiveTab::Chat {
                            let msg = model.input.value().to_string();
                            if let Err(e) = model.message_tx.send(Message::SendNetworkMessage(msg)) {
                                error!("Failed to send message: {}", e)
                            }
                            model.input.reset();
                        }
                        if model.active_tab == ActiveTab::Solitaire {
                            let msg = model.input.value().to_string();
                            if let Err(e) = model.message_tx.send(Message::SendGameMessage(msg)) {
                                error!("Failed to send log message: {}", e)
                            }
                            model.input.reset();
                        }
                    } else {
                        let username = format!("username:{}", model.input.value().to_string());
                        if let Err(e) = model.message_tx.send(Message::RegisterUser(username)) {
                            error!("Failed to send register message: {}", e)
                        }
                    }
                }
                KeyCode::Esc => {
                    model.input_mode = InputMode::Normal;
                }
                _ => {
                    model.input.handle_event(&Event::Key(key));
                }
            },
        },
        Message::RegisterUser(username) => {
            model.network_manager.send_message(username);
            model.is_user_registered = true;
            model.input.reset();
        }
        Message::ReceivedNetworkMessage(msg) => {
            model.messages.push(msg);
        }
        Message::SendNetworkMessage(msg) => {
            model.network_manager.send_message(msg);
        }
        Message::Log(msg) => {
            model.logs.push(msg);
        }

        Message::SendGameMessage(msg) => {
            //model.network_manager.send_message(msg);
            if model.active_tab == ActiveTab::Solitaire {
                if let Some(((from_row, from_col), (to_row, to_col))) = parse_move_command(&msg) {
                    // Verificando se o movimento é válido
                    if is_valid_move(from_row, from_col, to_row, to_col, &model.board) {
                        // Executando o movimento: movendo o pino e removendo o pino que foi pulado
                        make_move(from_row, from_col, to_row, to_col, &mut model.board);
                    }
                }
            }
        }
        _ => {}
    }
}
