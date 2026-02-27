use crossterm::event::{KeyCode, KeyEvent};
use rand::seq::SliceRandom;
use ratatui::{
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::games::Game;

// ---------------------------------------------------------------------------
// Card types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Suit {
    Hearts,
    Diamonds,
    Clubs,
    Spades,
}

impl Suit {
    fn symbol(self) -> &'static str {
        match self {
            Suit::Hearts => "\u{2665}",
            Suit::Diamonds => "\u{2666}",
            Suit::Clubs => "\u{2663}",
            Suit::Spades => "\u{2660}",
        }
    }

    fn color(self) -> Color {
        match self {
            Suit::Hearts | Suit::Diamonds => Color::Red,
            Suit::Clubs | Suit::Spades => Color::White,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Rank {
    Ace,
    Two,
    Three,
    Four,
    Five,
    Six,
    Seven,
    Eight,
    Nine,
    Ten,
    Jack,
    Queen,
    King,
}

impl Rank {
    fn label(self) -> &'static str {
        match self {
            Rank::Ace => "A",
            Rank::Two => "2",
            Rank::Three => "3",
            Rank::Four => "4",
            Rank::Five => "5",
            Rank::Six => "6",
            Rank::Seven => "7",
            Rank::Eight => "8",
            Rank::Nine => "9",
            Rank::Ten => "10",
            Rank::Jack => "J",
            Rank::Queen => "Q",
            Rank::King => "K",
        }
    }

    fn value(self) -> u8 {
        match self {
            Rank::Ace => 11,
            Rank::Two => 2,
            Rank::Three => 3,
            Rank::Four => 4,
            Rank::Five => 5,
            Rank::Six => 6,
            Rank::Seven => 7,
            Rank::Eight => 8,
            Rank::Nine => 9,
            Rank::Ten | Rank::Jack | Rank::Queen | Rank::King => 10,
        }
    }

    const ALL: [Rank; 13] = [
        Rank::Ace,
        Rank::Two,
        Rank::Three,
        Rank::Four,
        Rank::Five,
        Rank::Six,
        Rank::Seven,
        Rank::Eight,
        Rank::Nine,
        Rank::Ten,
        Rank::Jack,
        Rank::Queen,
        Rank::King,
    ];
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Card {
    rank: Rank,
    suit: Suit,
}

impl Card {
    /// Returns 7 lines of ASCII art for this card (width = 9 chars).
    fn art_lines(&self) -> Vec<String> {
        let r = self.rank.label();
        let s = self.suit.symbol();
        // Pad rank to 2 characters for alignment.
        let rl = format!("{:<2}", r);
        let rr = format!("{:>2}", r);
        vec![
            "\u{250c}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2510}".to_string(), // ┌───────┐
            format!("\u{2502} {}    \u{2502}", rl),                                                    // │ A     │
            format!("\u{2502}       \u{2502}"),                                                        // │       │
            format!("\u{2502}   {}   \u{2502}", s),                                                    // │   ♠   │
            format!("\u{2502}       \u{2502}"),                                                        // │       │
            format!("\u{2502}    {} \u{2502}", rr),                                                    // │    A  │
            "\u{2514}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2518}".to_string(), // └───────┘
        ]
    }

    fn suit_color(&self) -> Color {
        self.suit.color()
    }
}

/// Returns 7 lines of ASCII art for a face-down card.
fn facedown_art() -> Vec<String> {
    vec![
        "\u{250c}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2510}".to_string(),
        "\u{2502}\u{2591}\u{2591}\u{2591}\u{2591}\u{2591}\u{2591}\u{2591}\u{2502}".to_string(),
        "\u{2502}\u{2591}\u{2591}\u{2591}\u{2591}\u{2591}\u{2591}\u{2591}\u{2502}".to_string(),
        "\u{2502}\u{2591}\u{2591}\u{2591}\u{2591}\u{2591}\u{2591}\u{2591}\u{2502}".to_string(),
        "\u{2502}\u{2591}\u{2591}\u{2591}\u{2591}\u{2591}\u{2591}\u{2591}\u{2502}".to_string(),
        "\u{2502}\u{2591}\u{2591}\u{2591}\u{2591}\u{2591}\u{2591}\u{2591}\u{2502}".to_string(),
        "\u{2514}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2518}".to_string(),
    ]
}

// ---------------------------------------------------------------------------
// Deck
// ---------------------------------------------------------------------------

struct Deck {
    cards: Vec<Card>,
}

impl Deck {
    fn new_shuffled() -> Self {
        let mut cards = Vec::with_capacity(52);
        for &suit in &[Suit::Hearts, Suit::Diamonds, Suit::Clubs, Suit::Spades] {
            for &rank in &Rank::ALL {
                cards.push(Card { rank, suit });
            }
        }
        let mut rng = rand::thread_rng();
        cards.shuffle(&mut rng);
        Deck { cards }
    }

    fn draw(&mut self) -> Card {
        // If somehow exhausted, reshuffle a fresh deck.
        if self.cards.is_empty() {
            *self = Deck::new_shuffled();
        }
        self.cards.pop().expect("deck should not be empty")
    }
}

// ---------------------------------------------------------------------------
// Hand scoring
// ---------------------------------------------------------------------------

fn hand_value(hand: &[Card]) -> u32 {
    let mut total: u32 = 0;
    let mut aces: u32 = 0;
    for card in hand {
        total += card.rank.value() as u32;
        if card.rank == Rank::Ace {
            aces += 1;
        }
    }
    while total > 21 && aces > 0 {
        total -= 10;
        aces -= 1;
    }
    total
}

fn is_blackjack(hand: &[Card]) -> bool {
    hand.len() == 2 && hand_value(hand) == 21
}

// ---------------------------------------------------------------------------
// Game phases and result
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Phase {
    Betting,
    PlayerTurn,
    DealerTurn,
    RoundOver,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RoundResult {
    PlayerBlackjack,
    PlayerWin,
    DealerWin,
    Push,
}

impl RoundResult {
    fn label(self) -> &'static str {
        match self {
            RoundResult::PlayerBlackjack => "BLACKJACK!",
            RoundResult::PlayerWin => "YOU WIN!",
            RoundResult::DealerWin => "DEALER WINS",
            RoundResult::Push => "PUSH",
        }
    }

    fn color(self) -> Color {
        match self {
            RoundResult::PlayerBlackjack => Color::Magenta,
            RoundResult::PlayerWin => Color::Green,
            RoundResult::DealerWin => Color::Red,
            RoundResult::Push => Color::Yellow,
        }
    }
}

// ---------------------------------------------------------------------------
// Game state
// ---------------------------------------------------------------------------

pub struct BlackjackGame {
    deck: Deck,
    player_hand: Vec<Card>,
    dealer_hand: Vec<Card>,
    phase: Phase,
    result: Option<RoundResult>,
    chips: i32,
    bet: i32,
    bet_input: String,
    doubled: bool,
    message: String,
    /// Tick counter for dealer card reveal animation.
    dealer_tick: u32,
}

const STARTING_CHIPS: i32 = 100;
const MIN_BET: i32 = 1;
/// Ticks between each dealer card draw when playing out dealer hand.
const DEALER_DRAW_DELAY: u32 = 12;

// ---------------------------------------------------------------------------
// Trait implementation
// ---------------------------------------------------------------------------

impl Game for BlackjackGame {
    fn new() -> Self {
        BlackjackGame {
            deck: Deck::new_shuffled(),
            player_hand: Vec::new(),
            dealer_hand: Vec::new(),
            phase: Phase::Betting,
            result: None,
            chips: STARTING_CHIPS,
            bet: 0,
            bet_input: String::new(),
            doubled: false,
            message: String::from("Place your bet!"),
            dealer_tick: 0,
        }
    }

    fn handle_event(&mut self, event: KeyEvent) -> bool {
        match self.phase {
            Phase::Betting => self.handle_betting(event),
            Phase::PlayerTurn => self.handle_player_turn(event),
            Phase::DealerTurn => {
                // No player input during dealer play, but allow quit.
                if event.code == KeyCode::Esc {
                    return false;
                }
                true
            }
            Phase::RoundOver => self.handle_round_over(event),
        }
    }

    fn update(&mut self) {
        if self.phase == Phase::DealerTurn {
            self.dealer_tick += 1;
            if self.dealer_tick >= DEALER_DRAW_DELAY {
                self.dealer_tick = 0;
                self.dealer_step();
            }
        }
    }

    fn render(&self, frame: &mut Frame) {
        let area = frame.area();

        let outer = Block::default()
            .title(" Blackjack ")
            .title_alignment(Alignment::Center)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray));
        let inner = outer.inner(area);
        frame.render_widget(outer, area);

        // Fill background.
        let bg = Paragraph::new("")
            .style(Style::default().bg(Color::Rgb(0, 60, 30)));
        frame.render_widget(bg, inner);

        // Layout: top info | dealer hand | gap | player hand | bottom controls.
        let chunks = Layout::vertical([
            Constraint::Length(2),  // chip count + message
            Constraint::Length(9),  // dealer area (label + 7 card rows + score)
            Constraint::Length(1),  // separator
            Constraint::Length(9),  // player area
            Constraint::Min(0),    // controls / result
        ])
        .split(inner);

        self.render_info_bar(frame, chunks[0]);
        self.render_dealer(frame, chunks[1]);
        self.render_separator(frame, chunks[2]);
        self.render_player(frame, chunks[3]);
        self.render_controls(frame, chunks[4]);
    }
}

// ---------------------------------------------------------------------------
// Input handling
// ---------------------------------------------------------------------------

impl BlackjackGame {
    fn handle_betting(&mut self, event: KeyEvent) -> bool {
        match event.code {
            KeyCode::Esc => return false,
            KeyCode::Char(c) if c.is_ascii_digit() => {
                if self.bet_input.len() < 5 {
                    self.bet_input.push(c);
                    self.message = format!("Bet: {} chips", self.bet_input);
                }
            }
            KeyCode::Backspace => {
                self.bet_input.pop();
                if self.bet_input.is_empty() {
                    self.message = String::from("Place your bet!");
                } else {
                    self.message = format!("Bet: {} chips", self.bet_input);
                }
            }
            KeyCode::Enter => {
                if let Ok(b) = self.bet_input.parse::<i32>() {
                    if b < MIN_BET {
                        self.message = format!("Minimum bet is {} chip(s)!", MIN_BET);
                    } else if b > self.chips {
                        self.message = String::from("Not enough chips!");
                    } else {
                        self.bet = b;
                        self.bet_input.clear();
                        self.start_round();
                    }
                } else {
                    self.message = String::from("Enter a valid number!");
                }
            }
            _ => {}
        }
        true
    }

    fn handle_player_turn(&mut self, event: KeyEvent) -> bool {
        match event.code {
            KeyCode::Esc => return false,
            KeyCode::Char('h') | KeyCode::Char('H') => {
                self.player_hit();
            }
            KeyCode::Char('s') | KeyCode::Char('S') => {
                self.player_stand();
            }
            KeyCode::Char('d') | KeyCode::Char('D') => {
                self.player_double();
            }
            _ => {}
        }
        true
    }

    fn handle_round_over(&mut self, event: KeyEvent) -> bool {
        match event.code {
            KeyCode::Esc => return false,
            KeyCode::Char('n') | KeyCode::Char('N') => {
                if self.chips <= 0 {
                    // Reset to starting chips when broke.
                    self.chips = STARTING_CHIPS;
                }
                self.new_hand();
            }
            _ => {}
        }
        true
    }
}

// ---------------------------------------------------------------------------
// Game logic
// ---------------------------------------------------------------------------

impl BlackjackGame {
    fn start_round(&mut self) {
        self.player_hand.clear();
        self.dealer_hand.clear();
        self.result = None;
        self.doubled = false;
        self.dealer_tick = 0;

        // Deal: player, dealer, player, dealer.
        self.player_hand.push(self.deck.draw());
        self.dealer_hand.push(self.deck.draw());
        self.player_hand.push(self.deck.draw());
        self.dealer_hand.push(self.deck.draw());

        // Check for blackjacks.
        let player_bj = is_blackjack(&self.player_hand);
        let dealer_bj = is_blackjack(&self.dealer_hand);

        if player_bj && dealer_bj {
            self.finish_round(RoundResult::Push);
        } else if player_bj {
            self.finish_round(RoundResult::PlayerBlackjack);
        } else if dealer_bj {
            self.finish_round(RoundResult::DealerWin);
        } else {
            self.phase = Phase::PlayerTurn;
            self.message = String::from("[H]it  [S]tand  [D]ouble Down");
        }
    }

    fn player_hit(&mut self) {
        self.player_hand.push(self.deck.draw());
        let val = hand_value(&self.player_hand);
        if val > 21 {
            self.finish_round(RoundResult::DealerWin);
        } else if val == 21 {
            // Auto-stand on 21.
            self.player_stand();
        } else {
            self.message = String::from("[H]it  [S]tand");
        }
    }

    fn player_stand(&mut self) {
        self.phase = Phase::DealerTurn;
        self.dealer_tick = 0;
        self.message = String::from("Dealer's turn...");
    }

    fn player_double(&mut self) {
        // Double down: only allowed on first two cards and if player has enough chips.
        if self.player_hand.len() != 2 {
            self.message = String::from("Can only double on first two cards!");
            return;
        }
        if self.bet > self.chips - self.bet {
            // Not enough chips to double the bet; allow partial double up to remaining chips.
            if self.chips - self.bet <= 0 {
                self.message = String::from("Not enough chips to double!");
                return;
            }
        }
        self.doubled = true;
        let extra = self.bet.min(self.chips - self.bet);
        self.bet += extra;
        self.player_hand.push(self.deck.draw());
        let val = hand_value(&self.player_hand);
        if val > 21 {
            self.finish_round(RoundResult::DealerWin);
        } else {
            self.player_stand();
        }
    }

    /// Advance dealer play by one card / decision.
    fn dealer_step(&mut self) {
        let dealer_val = hand_value(&self.dealer_hand);
        if dealer_val < 17 {
            self.dealer_hand.push(self.deck.draw());
        } else {
            // Dealer stands; determine result.
            let player_val = hand_value(&self.player_hand);
            let final_dealer = hand_value(&self.dealer_hand);
            let result = if final_dealer > 21 {
                RoundResult::PlayerWin
            } else if player_val > final_dealer {
                RoundResult::PlayerWin
            } else if final_dealer > player_val {
                RoundResult::DealerWin
            } else {
                RoundResult::Push
            };
            self.finish_round(result);
        }
    }

    fn finish_round(&mut self, result: RoundResult) {
        self.phase = Phase::RoundOver;
        self.result = Some(result);
        match result {
            RoundResult::PlayerBlackjack => {
                // Blackjack pays 3:2.
                let winnings = self.bet + self.bet * 3 / 2;
                self.chips += winnings;
                self.message = format!("BLACKJACK! +{} chips", winnings);
            }
            RoundResult::PlayerWin => {
                self.chips += self.bet;
                self.message = format!("You win! +{} chips", self.bet);
            }
            RoundResult::DealerWin => {
                self.chips -= self.bet;
                self.message = format!("Dealer wins. -{} chips", self.bet);
            }
            RoundResult::Push => {
                self.message = String::from("Push - bet returned.");
            }
        }
    }

    fn new_hand(&mut self) {
        self.player_hand.clear();
        self.dealer_hand.clear();
        self.result = None;
        self.doubled = false;
        self.bet = 0;
        self.bet_input.clear();
        self.dealer_tick = 0;
        self.phase = Phase::Betting;
        // Re-shuffle if deck is getting low.
        if self.deck.cards.len() < 15 {
            self.deck = Deck::new_shuffled();
        }
        if self.chips <= 0 {
            self.message = String::from("Out of chips! Press 'n' to restart with 100 chips.");
        } else {
            self.message = String::from("Place your bet!");
        }
    }
}

// ---------------------------------------------------------------------------
// Rendering
// ---------------------------------------------------------------------------

impl BlackjackGame {
    fn render_info_bar(&self, frame: &mut Frame, area: Rect) {
        let chunks = Layout::horizontal([
            Constraint::Percentage(30),
            Constraint::Percentage(40),
            Constraint::Percentage(30),
        ])
        .split(area);

        // Chip count.
        let chip_color = if self.chips <= 0 {
            Color::Red
        } else if self.chips < STARTING_CHIPS {
            Color::Yellow
        } else {
            Color::Green
        };
        let chip_text = Paragraph::new(Line::from(vec![
            Span::styled(" Chips: ", Style::default().fg(Color::White)),
            Span::styled(
                format!("{}", self.chips),
                Style::default()
                    .fg(chip_color)
                    .add_modifier(Modifier::BOLD),
            ),
        ]))
        .style(Style::default().bg(Color::Rgb(0, 60, 30)));
        frame.render_widget(chip_text, chunks[0]);

        // Message.
        let msg_color = if let Some(ref r) = self.result {
            r.color()
        } else {
            Color::White
        };
        let msg = Paragraph::new(Line::from(Span::styled(
            &self.message,
            Style::default()
                .fg(msg_color)
                .add_modifier(Modifier::BOLD),
        )))
        .alignment(Alignment::Center)
        .style(Style::default().bg(Color::Rgb(0, 60, 30)));
        frame.render_widget(msg, chunks[1]);

        // Bet display.
        if self.bet > 0 {
            let bet_text = Paragraph::new(Line::from(vec![
                Span::styled("Bet: ", Style::default().fg(Color::White)),
                Span::styled(
                    format!("{}", self.bet),
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                ),
                if self.doubled {
                    Span::styled(" (doubled)", Style::default().fg(Color::Cyan))
                } else {
                    Span::raw("")
                },
            ]))
            .alignment(Alignment::Right)
            .style(Style::default().bg(Color::Rgb(0, 60, 30)));
            frame.render_widget(bet_text, chunks[2]);
        }
    }

    fn render_dealer(&self, frame: &mut Frame, area: Rect) {
        let chunks = Layout::vertical([
            Constraint::Length(1), // label + score
            Constraint::Length(7), // cards
            Constraint::Length(1), // score line
        ])
        .split(area);

        // Label.
        let show_all = self.phase == Phase::DealerTurn || self.phase == Phase::RoundOver;
        let dealer_score = if show_all {
            format!("  ({})", hand_value(&self.dealer_hand))
        } else if !self.dealer_hand.is_empty() {
            // Show only the up-card value.
            format!("  ({}+?)", self.dealer_hand[1].rank.value())
        } else {
            String::new()
        };

        let label = Paragraph::new(Line::from(vec![
            Span::styled(
                " DEALER",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(dealer_score, Style::default().fg(Color::Gray)),
        ]))
        .style(Style::default().bg(Color::Rgb(0, 60, 30)));
        frame.render_widget(label, chunks[0]);

        // Cards.
        self.render_hand(frame, chunks[1], &self.dealer_hand, !show_all);
    }

    fn render_separator(&self, frame: &mut Frame, area: Rect) {
        let sep_str: String = "\u{2500}".repeat(area.width as usize);
        let sep = Paragraph::new(Line::from(Span::styled(
            sep_str,
            Style::default().fg(Color::DarkGray),
        )))
        .style(Style::default().bg(Color::Rgb(0, 60, 30)));
        frame.render_widget(sep, area);
    }

    fn render_player(&self, frame: &mut Frame, area: Rect) {
        let chunks = Layout::vertical([
            Constraint::Length(1), // label + score
            Constraint::Length(7), // cards
            Constraint::Length(1), // padding
        ])
        .split(area);

        let player_score = if !self.player_hand.is_empty() {
            format!("  ({})", hand_value(&self.player_hand))
        } else {
            String::new()
        };

        let label = Paragraph::new(Line::from(vec![
            Span::styled(
                " PLAYER",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(player_score, Style::default().fg(Color::Gray)),
        ]))
        .style(Style::default().bg(Color::Rgb(0, 60, 30)));
        frame.render_widget(label, chunks[0]);

        self.render_hand(frame, chunks[1], &self.player_hand, false);
    }

    fn render_hand(
        &self,
        frame: &mut Frame,
        area: Rect,
        hand: &[Card],
        hide_first: bool,
    ) {
        if hand.is_empty() {
            return;
        }

        const CARD_WIDTH: u16 = 9;
        const CARD_HEIGHT: u16 = 7;
        const CARD_GAP: u16 = 1;

        // Compute total width needed.
        let num = hand.len() as u16;
        let total_w = num * CARD_WIDTH + num.saturating_sub(1) * CARD_GAP;

        // Centre the hand in the area.
        let start_x = area.x + area.width.saturating_sub(total_w) / 2;

        for (i, card) in hand.iter().enumerate() {
            let x = start_x + i as u16 * (CARD_WIDTH + CARD_GAP);
            let card_rect = Rect::new(
                x.min(area.x + area.width),
                area.y,
                CARD_WIDTH.min(area.x + area.width - x.min(area.x + area.width)),
                CARD_HEIGHT.min(area.height),
            );

            if card_rect.width == 0 || card_rect.height == 0 {
                continue;
            }

            let (lines_data, fg_color) = if hide_first && i == 0 {
                (facedown_art(), Color::Blue)
            } else {
                (card.art_lines(), card.suit_color())
            };

            let styled_lines: Vec<Line> = lines_data
                .iter()
                .map(|l| {
                    Line::from(Span::styled(
                        l.clone(),
                        Style::default().fg(fg_color),
                    ))
                })
                .collect();

            let card_widget = Paragraph::new(styled_lines)
                .style(Style::default().bg(Color::Rgb(0, 60, 30)));
            frame.render_widget(card_widget, card_rect);
        }
    }

    fn render_controls(&self, frame: &mut Frame, area: Rect) {
        let bg = Paragraph::new("")
            .style(Style::default().bg(Color::Rgb(0, 60, 30)));
        frame.render_widget(bg, area);

        match self.phase {
            Phase::Betting => {
                let input_display = if self.bet_input.is_empty() {
                    String::from("_")
                } else {
                    format!("{}_", self.bet_input)
                };

                let betting_lines = vec![
                    Line::from(""),
                    Line::from(vec![
                        Span::styled(
                            "  Enter bet amount: ",
                            Style::default().fg(Color::White),
                        ),
                        Span::styled(
                            input_display,
                            Style::default()
                                .fg(Color::Yellow)
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(
                            format!("  (1-{})", self.chips),
                            Style::default().fg(Color::DarkGray),
                        ),
                    ]),
                    Line::from(""),
                    Line::from(Span::styled(
                        "  Type a number and press Enter to deal. Esc to quit.",
                        Style::default().fg(Color::DarkGray),
                    )),
                ];
                let p = Paragraph::new(betting_lines)
                    .style(Style::default().bg(Color::Rgb(0, 60, 30)));
                frame.render_widget(p, area);
            }
            Phase::PlayerTurn => {
                let can_double = self.player_hand.len() == 2
                    && self.chips - self.bet > 0;
                let mut spans = vec![
                    Span::styled(
                        "  [H]",
                        Style::default()
                            .fg(Color::Green)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(" Hit  ", Style::default().fg(Color::White)),
                    Span::styled(
                        "[S]",
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(" Stand  ", Style::default().fg(Color::White)),
                ];
                if can_double {
                    spans.push(Span::styled(
                        "[D]",
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD),
                    ));
                    spans.push(Span::styled(
                        " Double Down  ",
                        Style::default().fg(Color::White),
                    ));
                }
                spans.push(Span::styled(
                    "[Esc]",
                    Style::default().fg(Color::Red),
                ));
                spans.push(Span::styled(" Quit", Style::default().fg(Color::DarkGray)));

                let controls = Paragraph::new(vec![Line::from(""), Line::from(spans)])
                    .style(Style::default().bg(Color::Rgb(0, 60, 30)));
                frame.render_widget(controls, area);
            }
            Phase::DealerTurn => {
                let p = Paragraph::new(vec![
                    Line::from(""),
                    Line::from(Span::styled(
                        "  Dealer is playing...",
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD),
                    )),
                ])
                .style(Style::default().bg(Color::Rgb(0, 60, 30)));
                frame.render_widget(p, area);
            }
            Phase::RoundOver => {
                let result_line = if let Some(r) = self.result {
                    Line::from(Span::styled(
                        format!("  {}", r.label()),
                        Style::default()
                            .fg(r.color())
                            .add_modifier(Modifier::BOLD),
                    ))
                } else {
                    Line::from("")
                };

                let next_msg = if self.chips <= 0 {
                    "  [N] New game (resets to 100 chips)  [Esc] Quit"
                } else {
                    "  [N] New hand  [Esc] Quit"
                };

                let lines = vec![
                    Line::from(""),
                    result_line,
                    Line::from(""),
                    Line::from(Span::styled(
                        next_msg,
                        Style::default().fg(Color::DarkGray),
                    )),
                ];
                let p = Paragraph::new(lines)
                    .style(Style::default().bg(Color::Rgb(0, 60, 30)));
                frame.render_widget(p, area);
            }
        }
    }
}

