//! Engine-independent game rules for Invaders.
//!
//! This module deliberately does not depend on Macroquad. Keeping the simulation
//! separate makes it possible to test the rules without opening a window.

pub const WORLD_WIDTH: f32 = 800.0;
pub const WORLD_HEIGHT: f32 = 600.0;
pub const HUD_HEIGHT: f32 = 64.0;
pub const WORLD_MARGIN: f32 = 16.0;

pub const PLAYER_WIDTH: f32 = 42.0;
pub const PLAYER_HEIGHT: f32 = 24.0;
pub const PLAYER_Y: f32 = 548.0;
pub const PLAYER_SPEED: f32 = 340.0;

pub const ALIEN_WIDTH: f32 = 36.0;
pub const ALIEN_HEIGHT: f32 = 24.0;
pub const ALIEN_STEP: f32 = 14.0;
pub const ALIEN_DESCENT: f32 = 22.0;

pub const PLAYER_BULLET_WIDTH: f32 = 4.0;
pub const PLAYER_BULLET_HEIGHT: f32 = 14.0;
pub const PLAYER_BULLET_SPEED: f32 = 620.0;
pub const MAX_PLAYER_BULLETS: usize = 6;

const ALIEN_COLUMNS: usize = 10;
const ALIEN_ROWS: usize = 5;
const ALIEN_START_X: f32 = 85.0;
const ALIEN_START_Y: f32 = 96.0;
const ALIEN_COLUMN_SPACING: f32 = 60.0;
const ALIEN_ROW_SPACING: f32 = 48.0;
const WAVE_CLEAR_DURATION: f32 = 1.25;
const WAVE_INTRO_DURATION: f32 = 1.6;

/// Points awarded for clearing a wave, before the wave scaling.
pub const WAVE_BONUS_POINTS: u32 = 50;
/// How many waves the formation may drift downwards before it stops.
const MAX_WAVE_DEPTH: u32 = 6;
/// The lowest row a formation may start on, leaving room to manoeuvre.
const MAX_FORMATION_BOTTOM: f32 = 430.0;

/// The difficulty knobs of the wave that is currently being played.
///
/// Every value is derived from the wave number, so a cleared wave always
/// hands the player a tougher formation than the previous one.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WaveDifficulty {
    /// Number of rows in the formation.
    pub rows: usize,
    /// How long the formation waits between steps.
    pub move_interval: f32,
    /// How long the invaders wait between volleys.
    pub fire_interval: f32,
    /// Speed of the invaders' projectiles.
    pub bullet_speed: f32,
    /// How many invaders fire in a single volley.
    pub volley_size: usize,
    /// How far the formation starts below its original position.
    pub start_depth: f32,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct InputState {
    pub move_left: bool,
    pub move_right: bool,
    pub fire: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GameState {
    Ready,
    Playing,
    Paused,
    GameOver,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GameEvent {
    PlayerFired,
    AlienFired,
    AlienDestroyed,
    PlayerHit,
    WaveCleared,
    GameOver,
}

#[derive(Clone, Copy, Debug)]
pub struct Player {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

#[derive(Clone, Copy, Debug)]
pub struct Alien {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub row: usize,
    pub alive: bool,
}

#[derive(Clone, Copy, Debug)]
pub struct Bullet {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub dead: bool,
}

#[derive(Clone, Copy, Debug)]
struct CollisionRect {
    x: f32,
    y: f32,
    width: f32,
    height: f32,
}

impl CollisionRect {
    fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    fn overlaps(self, other: Self) -> bool {
        self.x < other.x + other.width
            && self.x + self.width > other.x
            && self.y < other.y + other.height
            && self.y + self.height > other.y
    }
}

#[derive(Clone, Debug)]
pub struct Game {
    pub width: f32,
    pub height: f32,
    pub state: GameState,
    pub player: Player,
    pub aliens: Vec<Alien>,
    pub player_bullets: Vec<Bullet>,
    pub alien_bullets: Vec<Bullet>,
    pub alien_direction: f32,
    pub score: u32,
    pub wave: u32,
    pub lives: u8,
    pub elapsed: f32,
    pub wave_clear_timer: f32,
    pub wave_intro_timer: f32,
    pub invulnerability_timer: f32,

    events: Vec<GameEvent>,
    alien_move_timer: f32,
    alien_fire_timer: f32,
    player_fire_cooldown: f32,
    random_state: u64,
}

impl Game {
    pub fn new() -> Self {
        Self::new_with_seed(0x4d59_5df4_d0f3_3173)
    }

    pub fn new_with_seed(seed: u64) -> Self {
        let mut game = Self {
            width: WORLD_WIDTH,
            height: WORLD_HEIGHT,
            state: GameState::Ready,
            player: Player {
                x: WORLD_WIDTH / 2.0 - PLAYER_WIDTH / 2.0,
                y: PLAYER_Y,
                width: PLAYER_WIDTH,
                height: PLAYER_HEIGHT,
            },
            aliens: Vec::new(),
            player_bullets: Vec::new(),
            alien_bullets: Vec::new(),
            alien_direction: 1.0,
            score: 0,
            wave: 1,
            lives: 3,
            elapsed: 0.0,
            wave_clear_timer: 0.0,
            wave_intro_timer: 0.0,
            invulnerability_timer: 0.0,
            events: Vec::new(),
            alien_move_timer: 0.0,
            alien_fire_timer: 0.8,
            player_fire_cooldown: 0.0,
            random_state: if seed == 0 { 1 } else { seed },
        };
        game.spawn_wave();
        game
    }

    pub fn start(&mut self) {
        if self.state == GameState::Ready {
            self.state = GameState::Playing;
        }
    }

    pub fn restart(&mut self) {
        let next_seed = self
            .random_state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1);
        *self = Self::new_with_seed(next_seed);
        self.state = GameState::Playing;
    }

    pub fn toggle_pause(&mut self) {
        match self.state {
            GameState::Playing => self.state = GameState::Paused,
            GameState::Paused => self.state = GameState::Playing,
            GameState::Ready | GameState::GameOver => {}
        }
    }

    pub fn take_events(&mut self) -> Vec<GameEvent> {
        std::mem::take(&mut self.events)
    }

    pub fn update(&mut self, delta_time: f32, input: InputState) {
        if self.state != GameState::Playing {
            return;
        }

        let delta_time = delta_time.clamp(0.0, 0.05);
        self.elapsed += delta_time;
        self.invulnerability_timer = (self.invulnerability_timer - delta_time).max(0.0);
        self.player_fire_cooldown = (self.player_fire_cooldown - delta_time).max(0.0);

        self.update_player(delta_time, input);
        self.update_bullets(delta_time);
        self.resolve_player_bullet_hits();
        self.resolve_alien_bullet_hits();

        // The incoming wave is announced while the formation holds still, so
        // the player gets a short breather between waves.
        if self.wave_intro_timer > 0.0 {
            self.wave_intro_timer = (self.wave_intro_timer - delta_time).max(0.0);
            return;
        }

        self.update_alien_movement(delta_time);
        self.update_alien_firing(delta_time);

        if self.state != GameState::Playing {
            return;
        }

        if self
            .aliens
            .iter()
            .any(|alien| alien.alive && self.player_overlaps_alien(alien))
        {
            self.end_game();
            return;
        }

        if self.aliens.iter().all(|alien| !alien.alive) {
            self.wave_clear_timer += delta_time;
            self.player_bullets.clear();
            self.alien_bullets.clear();

            if self.wave_clear_timer >= WAVE_CLEAR_DURATION {
                self.advance_wave();
            }
        }
    }

    pub fn wave_progress(&self) -> f32 {
        (self.wave_clear_timer / WAVE_CLEAR_DURATION).clamp(0.0, 1.0)
    }

    /// Remaining time of the "wave incoming" announcement.
    pub fn wave_intro_remaining(&self) -> f32 {
        self.wave_intro_timer
    }

    /// Fraction of the announcement that is still left.
    pub fn wave_intro_progress(&self) -> f32 {
        (self.wave_intro_timer / WAVE_INTRO_DURATION).clamp(0.0, 1.0)
    }

    /// Number of invaders still standing in the current wave.
    pub fn aliens_remaining(&self) -> usize {
        self.aliens.iter().filter(|alien| alien.alive).count()
    }

    pub fn spawn_wave(&mut self) {
        self.aliens.clear();

        let difficulty = self.difficulty();
        for row in 0..difficulty.rows {
            for column in 0..ALIEN_COLUMNS {
                self.aliens.push(Alien {
                    x: ALIEN_START_X + column as f32 * ALIEN_COLUMN_SPACING,
                    y: ALIEN_START_Y + difficulty.start_depth + row as f32 * ALIEN_ROW_SPACING,
                    width: ALIEN_WIDTH,
                    height: ALIEN_HEIGHT,
                    row,
                    alive: true,
                });
            }
        }

        self.alien_direction = 1.0;
        self.alien_move_timer = 0.0;
        self.alien_fire_timer = 1.2;
        self.wave_clear_timer = 0.0;
    }

    fn advance_wave(&mut self) {
        self.events.push(GameEvent::WaveCleared);
        let bonus = self.wave_bonus();
        self.wave = self.wave.saturating_add(1);
        self.score = self.score.saturating_add(bonus);
        self.spawn_wave();
        self.wave_intro_timer = WAVE_INTRO_DURATION;
    }

    /// Difficulty of the wave that is currently being played.
    ///
    /// The curve is deliberately steep: later waves bring more rows, a faster
    /// formation, more simultaneous shots and faster projectiles.
    pub fn difficulty(&self) -> WaveDifficulty {
        let wave = self.wave.max(1);
        let steps = (wave - 1) as f32;
        let score_factor = self.score as f32 / 5_000.0;

        let rows = match wave {
            1..=2 => ALIEN_ROWS,
            3..=5 => ALIEN_ROWS + 1,
            _ => ALIEN_ROWS + 2,
        };

        // A deeper formation is only allowed while it still leaves the player
        // room to dodge, so bigger formations start closer to the top.
        let formation_bottom = ALIEN_START_Y + (rows as f32 - 1.0) * ALIEN_ROW_SPACING;
        let max_depth = (MAX_FORMATION_BOTTOM - formation_bottom).max(0.0);
        let start_depth = (steps.min(MAX_WAVE_DEPTH as f32) * 20.0).min(max_depth);

        WaveDifficulty {
            rows,
            move_interval: (0.86 - steps * 0.06 - score_factor * 0.04).max(0.14),
            fire_interval: (1.45 - steps * 0.08 - score_factor * 0.08).max(0.30),
            bullet_speed: 220.0 + steps * 14.0,
            volley_size: match wave {
                1..=2 => 1,
                3..=5 => 2,
                _ => 3,
            },
            start_depth,
        }
    }

    /// Points awarded for clearing the wave that was just finished.
    pub fn wave_bonus(&self) -> u32 {
        WAVE_BONUS_POINTS + (self.wave.max(1) - 1) * 25
    }

    /// How long the formation waits between steps.
    fn alien_move_interval(&self) -> f32 {
        self.difficulty().move_interval
    }

    /// How long the invaders wait between volleys.
    fn alien_fire_interval(&self) -> f32 {
        self.difficulty().fire_interval
    }

    /// Speed of the invaders' projectiles.
    fn alien_bullet_speed(&self) -> f32 {
        self.difficulty().bullet_speed
    }

    fn update_player(&mut self, delta_time: f32, input: InputState) {
        let direction = match (input.move_left, input.move_right) {
            (true, false) => -1.0,
            (false, true) => 1.0,
            _ => 0.0,
        };

        self.player.x += direction * PLAYER_SPEED * delta_time;
        let minimum_x = WORLD_MARGIN;
        let maximum_x = self.width - self.player.width - WORLD_MARGIN;
        self.player.x = self.player.x.clamp(minimum_x, maximum_x);

        if input.fire
            && self.wave_intro_timer <= 0.0
            && self.player_fire_cooldown <= 0.0
            && self.player_bullets.len() < MAX_PLAYER_BULLETS
        {
            self.player_bullets.push(Bullet {
                x: self.player.x + self.player.width / 2.0 - PLAYER_BULLET_WIDTH / 2.0,
                y: self.player.y - PLAYER_BULLET_HEIGHT,
                width: PLAYER_BULLET_WIDTH,
                height: PLAYER_BULLET_HEIGHT,
                dead: false,
            });
            self.events.push(GameEvent::PlayerFired);
            self.player_fire_cooldown = 0.2;
        }
    }

    fn update_bullets(&mut self, delta_time: f32) {
        for bullet in &mut self.player_bullets {
            bullet.y -= PLAYER_BULLET_SPEED * delta_time;
        }
        self.player_bullets
            .retain(|bullet| !bullet.dead && bullet.y + bullet.height >= HUD_HEIGHT);

        let alien_bullet_speed = self.alien_bullet_speed();
        for bullet in &mut self.alien_bullets {
            bullet.y += alien_bullet_speed * delta_time;
        }
        self.alien_bullets
            .retain(|bullet| !bullet.dead && bullet.y <= self.height);
    }

    fn resolve_player_bullet_hits(&mut self) {
        let mut kills = 0_u32;

        for bullet in &mut self.player_bullets {
            if bullet.dead {
                continue;
            }

            let hit = self.aliens.iter().enumerate().find_map(|(index, alien)| {
                if alien.alive
                    && CollisionRect::new(bullet.x, bullet.y, bullet.width, bullet.height).overlaps(
                        CollisionRect::new(alien.x, alien.y, alien.width, alien.height),
                    )
                {
                    Some(index)
                } else {
                    None
                }
            });

            if let Some(index) = hit {
                self.aliens[index].alive = false;
                bullet.dead = true;
                self.events.push(GameEvent::AlienDestroyed);
                kills += 1;
            }
        }

        self.score = self.score.saturating_add(kills * 10);
        self.player_bullets.retain(|bullet| !bullet.dead);
    }

    fn resolve_alien_bullet_hits(&mut self) {
        if self.invulnerability_timer > 0.0 {
            return;
        }

        let hit = self.alien_bullets.iter().position(|bullet| {
            CollisionRect::new(bullet.x, bullet.y, bullet.width, bullet.height).overlaps(
                CollisionRect::new(
                    self.player.x,
                    self.player.y,
                    self.player.width,
                    self.player.height,
                ),
            )
        });

        if let Some(index) = hit {
            self.alien_bullets.remove(index);
            self.player_hit();
        }
    }

    fn player_hit(&mut self) {
        if self.invulnerability_timer > 0.0 {
            return;
        }

        self.lives = self.lives.saturating_sub(1);
        self.events.push(GameEvent::PlayerHit);
        if self.lives == 0 {
            self.end_game();
            return;
        }

        self.player.x = self.width / 2.0 - self.player.width / 2.0;
        self.player_bullets.clear();
        self.alien_bullets.clear();
        self.invulnerability_timer = 1.5;
    }

    fn player_overlaps_alien(&self, alien: &Alien) -> bool {
        CollisionRect::new(
            self.player.x,
            self.player.y,
            self.player.width,
            self.player.height,
        )
        .overlaps(CollisionRect::new(
            alien.x,
            alien.y,
            alien.width,
            alien.height,
        ))
    }

    fn update_alien_movement(&mut self, delta_time: f32) {
        self.alien_move_timer -= delta_time;
        if self.alien_move_timer > 0.0 {
            return;
        }

        let movement_interval = self.alien_move_interval();
        self.alien_move_timer = movement_interval;

        let Some(minimum_x) = self
            .aliens
            .iter()
            .filter(|alien| alien.alive)
            .map(|alien| alien.x)
            .min_by(f32::total_cmp)
        else {
            return;
        };
        let Some(maximum_x) = self
            .aliens
            .iter()
            .filter(|alien| alien.alive)
            .map(|alien| alien.x + alien.width)
            .max_by(f32::total_cmp)
        else {
            return;
        };

        let hits_edge = if self.alien_direction > 0.0 {
            maximum_x + ALIEN_STEP >= self.width - WORLD_MARGIN
        } else {
            minimum_x - ALIEN_STEP <= WORLD_MARGIN
        };

        if hits_edge {
            self.alien_direction *= -1.0;
            for alien in self.aliens.iter_mut().filter(|alien| alien.alive) {
                alien.y += ALIEN_DESCENT;
            }
        } else {
            for alien in self.aliens.iter_mut().filter(|alien| alien.alive) {
                alien.x += self.alien_direction * ALIEN_STEP;
            }
        }

        if self
            .aliens
            .iter()
            .any(|alien| alien.alive && alien.y + alien.height >= self.player.y - 4.0)
        {
            self.end_game();
        }
    }

    fn update_alien_firing(&mut self, delta_time: f32) {
        self.alien_fire_timer -= delta_time;
        if self.alien_fire_timer > 0.0 {
            return;
        }

        let living_indices: Vec<usize> = self
            .aliens
            .iter()
            .enumerate()
            .filter_map(|(index, alien)| alien.alive.then_some(index))
            .collect();
        if living_indices.is_empty() {
            return;
        }

        // Later waves answer with a volley from several invaders at once.
        let volley_size = self
            .difficulty()
            .volley_size
            .min(living_indices.len())
            .max(1);

        let mut shooters: Vec<usize> = Vec::with_capacity(volley_size);
        for _ in 0..volley_size {
            let candidate = living_indices[self.random_index(living_indices.len())];
            if !shooters.contains(&candidate) {
                shooters.push(candidate);
            }
        }

        for index in shooters {
            let alien = self.aliens[index];
            self.alien_bullets.push(Bullet {
                x: alien.x + alien.width / 2.0 - 2.0,
                y: alien.y + alien.height,
                width: 4.0,
                height: 10.0,
                dead: false,
            });
            self.events.push(GameEvent::AlienFired);
        }

        let base_interval = self.alien_fire_interval();
        let variation = 0.8 + (self.next_random() % 401) as f32 / 1_000.0;
        self.alien_fire_timer = base_interval * variation;
    }

    fn random_index(&mut self, length: usize) -> usize {
        if length == 0 {
            return 0;
        }
        (self.next_random() as usize) % length
    }

    fn next_random(&mut self) -> u32 {
        let mut value = self.random_state;
        value ^= value << 13;
        value ^= value >> 7;
        value ^= value << 17;
        self.random_state = value;
        (value >> 32) as u32
    }

    fn end_game(&mut self) {
        if self.state != GameState::GameOver {
            self.events.push(GameEvent::GameOver);
        }
        self.state = GameState::GameOver;
    }
}

impl Default for Game {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_game_contains_a_complete_wave() {
        let game = Game::new();
        assert_eq!(game.state, GameState::Ready);
        assert_eq!(game.wave, 1);
        assert_eq!(game.aliens.len(), ALIEN_COLUMNS * ALIEN_ROWS);
        assert!(game.aliens.iter().all(|alien| alien.alive));
    }

    #[test]
    fn player_stays_inside_the_playfield() {
        let mut game = Game::new();
        game.start();

        game.player.x = -100.0;
        game.update(
            1.0,
            InputState {
                move_left: true,
                ..InputState::default()
            },
        );
        assert_eq!(game.player.x, WORLD_MARGIN);

        game.player.x = game.width;
        game.update(
            1.0,
            InputState {
                move_right: true,
                ..InputState::default()
            },
        );
        assert_eq!(game.player.x, game.width - PLAYER_WIDTH - WORLD_MARGIN);
    }

    #[test]
    fn player_bullet_kills_an_alien_and_scores_points() {
        let mut game = Game::new();
        game.start();
        game.aliens.truncate(1);

        let alien = game.aliens[0];
        game.player_bullets.push(Bullet {
            x: alien.x + 10.0,
            y: alien.y + 5.0,
            width: PLAYER_BULLET_WIDTH,
            height: PLAYER_BULLET_HEIGHT,
            dead: false,
        });

        game.resolve_player_bullet_hits();

        assert!(!game.aliens[0].alive);
        assert_eq!(game.score, 10);
        assert!(game.player_bullets.is_empty());
    }

    #[test]
    fn aliens_reverse_and_descend_at_the_edge() {
        let mut game = Game::new();
        game.start();
        game.aliens.truncate(1);
        game.aliens[0].x = game.width - ALIEN_WIDTH - 10.0;
        let original_y = game.aliens[0].y;

        game.update_alien_movement(1.0);

        assert_eq!(game.alien_direction, -1.0);
        assert!(game.aliens[0].y > original_y);
    }

    #[test]
    fn an_alien_bullet_consumes_one_life() {
        let mut game = Game::new();
        game.start();
        game.alien_bullets.push(Bullet {
            x: game.player.x,
            y: game.player.y,
            width: 4.0,
            height: 10.0,
            dead: false,
        });

        game.resolve_alien_bullet_hits();

        assert_eq!(game.lives, 2);
        assert_eq!(game.state, GameState::Playing);
        assert!(game.alien_bullets.is_empty());
    }

    #[test]
    fn clearing_a_wave_advances_to_the_next_wave() {
        let mut game = Game::new();
        game.start();
        for alien in &mut game.aliens {
            alien.alive = false;
        }
        game.wave_clear_timer = WAVE_CLEAR_DURATION - 0.01;

        game.update(0.02, InputState::default());

        assert_eq!(game.wave, 2);
        assert!(game.aliens.iter().all(|alien| alien.alive));
        assert_eq!(game.score, WAVE_BONUS_POINTS);
    }

    #[test]
    fn the_next_wave_is_announced_before_the_formation_moves() {
        let mut game = Game::new();
        game.start();
        for alien in &mut game.aliens {
            alien.alive = false;
        }
        game.wave_clear_timer = WAVE_CLEAR_DURATION - 0.01;
        game.update(0.02, InputState::default());

        assert_eq!(game.wave, 2);
        assert_eq!(game.wave_intro_remaining(), WAVE_INTRO_DURATION);

        let first_alien_x = game.aliens[0].x;
        game.update(0.5, InputState::default());
        assert_eq!(
            game.aliens[0].x, first_alien_x,
            "formation should hold still"
        );

        advance(&mut game, WAVE_INTRO_DURATION);
        assert_eq!(game.wave_intro_remaining(), 0.0);
        advance(&mut game, 1.0);
        assert_ne!(
            game.aliens[0].x, first_alien_x,
            "formation should move again"
        );
    }

    #[test]
    fn the_player_cannot_fire_during_the_wave_intro() {
        let mut game = Game::new();
        game.start();
        for alien in &mut game.aliens {
            alien.alive = false;
        }
        game.wave_clear_timer = WAVE_CLEAR_DURATION - 0.01;
        game.update(0.02, InputState::default());

        game.update(
            0.1,
            InputState {
                fire: true,
                ..InputState::default()
            },
        );
        assert!(game.player_bullets.is_empty());

        advance(&mut game, WAVE_INTRO_DURATION);
        game.update(
            0.1,
            InputState {
                fire: true,
                ..InputState::default()
            },
        );
        assert_eq!(game.player_bullets.len(), 1);
    }

    /// Runs the simulation in fixed steps, the way the game loop does.
    fn advance(game: &mut Game, seconds: f32) {
        let steps = (seconds / (1.0 / 60.0)).ceil() as u32;
        for _ in 0..steps {
            game.update(1.0 / 60.0, InputState::default());
        }
    }

    #[test]
    fn later_waves_are_more_aggressive() {
        let mut game = Game::new();

        let first = game.difficulty();

        game.wave = 6;
        let sixth = game.difficulty();

        assert!(sixth.move_interval < first.move_interval);
        assert!(sixth.fire_interval < first.fire_interval);
        assert!(sixth.bullet_speed > first.bullet_speed);
        assert!(sixth.volley_size > first.volley_size);
        assert!(sixth.rows > first.rows);
    }

    #[test]
    fn waves_grow_the_formation() {
        let mut game = Game::new();
        assert_eq!(game.difficulty().rows, ALIEN_ROWS);

        game.wave = 4;
        assert_eq!(game.difficulty().rows, ALIEN_ROWS + 1);

        game.wave = 7;
        assert_eq!(game.difficulty().rows, ALIEN_ROWS + 2);

        game.wave = 4;
        game.spawn_wave();
        assert_eq!(game.aliens.len(), (ALIEN_ROWS + 1) * ALIEN_COLUMNS);
    }

    #[test]
    fn the_formation_always_starts_above_the_player() {
        let mut game = Game::new();

        for wave in 1..=20 {
            game.wave = wave;
            game.spawn_wave();

            let lowest = game
                .aliens
                .iter()
                .map(|alien| alien.y + alien.height)
                .fold(0.0_f32, f32::max);

            assert!(
                lowest < game.player.y - 40.0,
                "wave {wave} starts too low: {lowest}"
            );
        }
    }

    #[test]
    fn the_wave_bonus_grows_with_the_wave() {
        let mut game = Game::new();
        assert_eq!(game.wave_bonus(), WAVE_BONUS_POINTS);

        game.wave = 4;
        assert_eq!(game.wave_bonus(), WAVE_BONUS_POINTS + 75);
    }

    #[test]
    fn waves_formation_starts_lower_as_the_game_progresses() {
        let mut game = Game::new();
        let first_wave_y = game.aliens[0].y;

        game.wave = 4;
        game.spawn_wave();

        assert!(game.aliens[0].y > first_wave_y);
    }
}
