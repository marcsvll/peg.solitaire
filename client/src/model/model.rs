use color_eyre::eyre::Result;
use ratatui::widgets::ListItem;
use tui_input::Input;

use crate::{update, view, FpsCounter, Message, NetworkManager, Tui};

#[derive(PartialEq, Eq)]
pub enum InputMode {
    Normal,
    Editing,
}
#[derive(PartialEq, Eq)]
pub enum ActiveTab {
    Chat,
    Logs,
    Solitaire,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SolitarieCell {
    Empty,
    Peg,
    Invalid, // Representa as partes do tabuleiro que não são utilizadas no layout do Peg Solitaire
}

impl ActiveTab {
    pub fn get_idx(&self) -> usize {
        match self {
            ActiveTab::Chat => 0,
            ActiveTab::Logs => 1,
            ActiveTab::Solitaire => 2,
        }
    }
}

// Model state
pub struct Model<'a> {
    pub message_tx: tokio::sync::mpsc::UnboundedSender<Message>,
    pub fps_counter: FpsCounter,
    pub input: Input,
    pub input_mode: InputMode,
    pub messages: Vec<String>,
    pub network_manager: NetworkManager,
    pub active_tab: ActiveTab,
    pub logs: Vec<ListItem<'a>>,
    pub is_user_registered: bool,
    pub board: Vec<Vec<SolitarieCell>>,
}

impl<'a> Model<'a> {
    pub fn new(tui: &Tui, network_manager: NetworkManager) -> Self {
        let mut board = vec![vec![SolitarieCell::Invalid; 7]; 7]; // 7x7 é um tamanho comum para o tabuleiro de Peg Solitaire
        
        // Definir as células válidas e o layout inicial do tabuleiro
        for i in 0..7 {
            for j in 0..7 {
                if (i >= 2 && i <= 4) || (j >= 2 && j <= 4) {
                    board[i][j] = SolitarieCell::Peg;
                }
            }
        }
        
        // Definir o espaço vazio no centro
        board[3][3] = SolitarieCell::Empty;

        Self {
            message_tx: tui.event_tx.clone(),
            fps_counter: FpsCounter::new(),
            input: Input::default(),
            input_mode: InputMode::Normal,
            messages: Vec::new(),
            network_manager,
            active_tab: ActiveTab::Chat,
            logs: Vec::new(),
            is_user_registered: false,
            board,
        }
    }

    pub async fn start(mut self, mut tui: Tui) -> Result<()> {
        tui.enter()?;
        let mut should_exit = false;
        loop {
            tokio::select! {
                Some(message) = tui.next() => {
                    match message {
                        Message::Render => {
                            // Update FPS counter
                            self.fps_counter.tick();
                            // Handle the render event
                            tui.terminal.draw(|f| {
                                view(f, &self);
                            })?;
                        },
                        Message::Quit => {
                            should_exit = true;
                        },
                        message => {
                            update(&mut self, message);
                        }
                    }
                },
                Some(network_msg) = self.network_manager.get_incoming_messages().recv() => {
                    update(&mut self, Message::ReceivedNetworkMessage(network_msg));
                },
            }
            if should_exit {
                break;
            }
        }

        tui.exit()?;
        Ok(())
    }
}
