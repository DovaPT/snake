#![allow(dead_code)]
use std::{
    collections::LinkedList,
    io::{self, Stdout, Write},
    ops,
    sync::mpsc::{self, Receiver, SyncSender},
    thread,
    time::{Duration, Instant},
};

use termion::{
    event::Key, input::TermRead, raw::IntoRawMode, screen::IntoAlternateScreen, terminal_size,
};

fn main() {
    thread::scope(|scope| {
        let (sender, reciever) = mpsc::sync_channel(0);
        scope.spawn(|| game_loop(reciever));

        scope.spawn(|| handle_input(sender));
    });
}

fn handle_input(sender: SyncSender<Commands>) {
    let mut key_reader = io::stdin().keys();
    while let Some(Ok(key)) = key_reader.next() {
        let Some(command) = Commands::from_key(key) else {continue};
        if sender.send(command).is_err() || matches!(command, Commands::Quit)
        {
            break;
        }
    }
}

fn game_loop(reciever: Receiver<Commands>) {
    let mut stdout = io::stdout()
        .into_raw_mode()
        .unwrap()
        .into_alternate_screen()
        .unwrap();
    let mut game = Game::new();
    let mut clock = Clock::new();
    game.draw(&mut stdout);
    let mut dt = 0.;
    loop {
        match reciever.try_recv() {
            Ok(cmd) => match cmd {
                Commands::RotatePlayer(dir) => {
                    game.player.rotate(dir);
                }
                Commands::Extend => game.player.extend(),
                Commands::Shrink => game.player.shrink(),
                Commands::Quit => break,
            },
            Err(mpsc::TryRecvError::Empty) => {}
            Err(mpsc::TryRecvError::Disconnected) => break,
        };
        game.update(dt);
        game.draw(&mut stdout);
        dt = clock.tick(30.);
    }
}

#[derive(Clone, Copy)]
enum Commands {
    RotatePlayer(f64),
    Extend,
    Shrink,
    Quit,
}

impl Commands {
    fn from_key(key: Key) -> Option<Commands> {
        match key {
            Key::Char('q') => Some(Commands::Quit),
            Key::Char('e') => Some(Commands::Extend),
            Key::Char('r') => Some(Commands::Shrink),
            Key::Right | Key::Char('d') | Key::Char('l') => {
                Some(Commands::RotatePlayer(90_f64.to_radians()))
            }
            Key::Left | Key::Char('a') | Key::Char('h') => {
                Some(Commands::RotatePlayer(-90_f64.to_radians()))
            }
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct Vec2 {
    x: f64,
    y: f64,
}

impl ops::Sub for Vec2 {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        let x = self.x - rhs.x;
        let y = self.y - rhs.y;
        Self { x, y }
    }
}

impl ops::AddAssign for Vec2 {
    fn add_assign(&mut self, rhs: Self) {
        self.x += rhs.x;
        self.y += rhs.y;
    }
}
impl ops::SubAssign for Vec2 {
    fn sub_assign(&mut self, rhs: Self) {
        self.x -= rhs.x;
        self.y -= rhs.y;
    }
}

impl ops::Mul<f64> for Vec2 {
    type Output = Self;

    fn mul(self, rhs: f64) -> Self::Output {
        let x = self.x * rhs;
        let y = self.y * rhs;
        Self { x, y }
    }
}

impl ops::Div for Vec2 {
    type Output = Self;

    fn div(self, rhs: Self) -> Self::Output {
        let x = self.x / rhs.x;
        let y = self.y / rhs.y;
        Self { x, y }
    }
}

impl ops::Add for Vec2 {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        let x = self.x + rhs.x;
        let y = self.y + rhs.y;
        Self::Output { x, y }
    }
}

impl Vec2 {
    pub fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }
    pub fn rotate(&mut self, angle: f64) {
        (self.x, self.y) = (
            self.x * angle.cos() - self.y * angle.sin(),
            self.x * angle.sin() + self.y * angle.cos(),
        )
    }
    pub fn clamp(mut self, min: Self, max: Self) -> Self {
        self.x = self.x.clamp(min.x, max.x);
        self.y = self.y.clamp(min.y, max.y);
        self
    }
    pub fn inside_rectange(&self, p1: Vec2, p2: Vec2) -> bool {
        return self.x >= p1.x && self.y >= p1.y && self.x <= p2.x && self.y <= p2.y;
    }

    pub fn round(self) -> Self {
        let x = self.x.round();
        let y = self.y.round();
        Self { x, y }
    }

    fn outside_rectange(&self, p1: Vec2, p2: Vec2) -> bool {
        return self.x < p1.x && self.y < p1.y && self.x > p2.x && self.y > p2.y;
    }
}

#[derive(Debug, Clone)]
struct Clock {
    last_tick: Instant,
}

impl Clock {
    fn new() -> Self {
        let last_tick = Instant::now();
        Self { last_tick }
    }
    fn tick(&mut self, fps: f64) -> f64 {
        let mut elapsed = self.last_tick.elapsed();
        if elapsed.as_secs_f64() <= 1. / fps {
            thread::sleep(Duration::from_secs_f64(1. / fps));
            elapsed = self.last_tick.elapsed()
        }
        self.last_tick = Instant::now();
        elapsed.as_secs_f64()
    }
}

#[derive(Debug, Clone)]
struct Snake {
    len: u32,
    head: Vec2,
    body: LinkedList<Vec2>,
    forward: Vec2,
}

impl Snake {
    pub fn new() -> Self {
        let len = 1;
        let head = Vec2::new(0.03, 0.03);
        let forward = Vec2::new(0.11, 0.);
        let body = LinkedList::new();
        Snake {
            len,
            head,
            forward,
            body,
        }
    }

    pub fn extend(&mut self) {
        let newhead = self.head
            + self
                .forward
                .clamp(Vec2::new(-0.01, -0.01), Vec2::new(0.01, 0.01));
        self.body.push_front(self.head);
        self.head = newhead;
    }

    pub fn shrink(&mut self) {
        self.body.pop_back();
    }

    pub fn r#move(&mut self, dt: f64) {
        self.body.push_front(self.head);
        self.head += self.forward * dt;
        self.body.pop_back();
    }

    pub fn rotate(&mut self, angle: f64) {
        self.forward.rotate(angle);
    }

    fn move_back(&mut self) {
        self.head -= self.forward;
    }
}

#[derive(Debug, Clone)]
struct Game {
    height: u16,
    width: u16,
    player: Snake,
    clock: Clock,
}

impl Game {
    fn new() -> Self {
        let (width, height) = terminal_size().unwrap();
        let player = Snake::new();
        let clock = Clock::new();
        Self {
            height,
            width,
            player,
            clock,
        }
    }

    fn update(&mut self, dt: f64) {
        if (self.player.head + self.player.forward * dt)
            .inside_rectange(Vec2::new(0., 0.), Vec2::new(1., 1.))
        {
            self.player.r#move(dt);
        } else if self
            .player
            .head
            .outside_rectange(Vec2::new(0., 0.), Vec2::new(1., 1.))
        {
        }
    }

    fn draw(&self, stdout: &mut termion::raw::RawTerminal<Stdout>) {
        write!(
            stdout,
            "{}{}",
            termion::clear::All,
            termion::cursor::Goto(1, 1)
        )
        .unwrap();
        writeln!(
            stdout,
            "snake head gamecoord: ({:0.2},{:0.2})",
            self.player.head.x, self.player.head.y
        )
        .unwrap();
        let snake_termcoord = self.term_coord(self.player.head);
        writeln!(
            stdout,
            "\rsnake head termcoord: ({},{})",
            snake_termcoord.0, snake_termcoord.1
        )
        .unwrap();
        self.draw_snake(stdout);
        stdout.flush().unwrap();
    }

    fn term_coord(&self, v: Vec2) -> (u16, u16) {
        let x = v.x * self.width as f64;
        let y = v.y * self.height as f64;
        return (x as u16 + 1, y as u16 + 1);
    }

    pub fn draw_snake(&self, stdout: &mut termion::raw::RawTerminal<Stdout>) {
        let (mut row, mut col) = self.term_coord(self.player.head);
        write!(
            stdout,
            "{}\u{2588}{}",
            termion::cursor::Goto(row, col),
            termion::cursor::Hide,
        )
        .unwrap();

        for peice in self.player.body.iter() {
            (row, col) = self.term_coord(*peice);
            write!(
                stdout,
                "{}\u{2588}{}",
                termion::cursor::Goto(row, col),
                termion::cursor::Hide,
            )
            .unwrap();
        }
    }

    fn game_coord(&self, x: u16, y: u16) -> Vec2 {
        let ratio = self.width as f64 / self.height as f64;
        let x = x as f64 * ratio;
        let y = y as f64 * ratio;
        return Vec2 { x, y };
    }
}
