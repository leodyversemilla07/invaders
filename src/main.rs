mod audio;
mod game;
mod render;

use std::error::Error;

use macroquad::prelude::*;

use crate::{
    audio::{AudioPlayer, Sound},
    game::{Game, GameEvent, GameState, InputState, WORLD_HEIGHT, WORLD_WIDTH},
    render::RenderAssets,
};

type AppResult<T> = Result<T, Box<dyn Error>>;

const FIXED_TIME_STEP: f32 = 1.0 / 60.0;

/// Lets the web page hide its loading overlay once the first frame is on screen.
#[cfg(target_arch = "wasm32")]
fn notify_web_ready() {
    #[link(wasm_import_module = "env")]
    unsafe extern "C" {
        fn game_ready();
    }

    unsafe { game_ready() };
}

#[cfg(not(target_arch = "wasm32"))]
fn notify_web_ready() {}

fn window_conf() -> Conf {
    Conf {
        window_title: "Invader Patrol".to_owned(),
        window_width: WORLD_WIDTH as i32,
        window_height: WORLD_HEIGHT as i32,
        fullscreen: false,
        ..Default::default()
    }
}

struct Sounds {
    player_shot: Option<Sound>,
    alien_shot: Option<Sound>,
    explosion: Option<Sound>,
    player_hit: Option<Sound>,
    wave_clear: Option<Sound>,
    game_over: Option<Sound>,
}

impl Sounds {
    async fn load(player: &AudioPlayer) -> Self {
        Self {
            player_shot: player.load("sounds/shoot.wav").await,
            alien_shot: player.load("sounds/alien_shoot.wav").await,
            explosion: player.load("sounds/explosion.wav").await,
            player_hit: player.load("sounds/player_hit.wav").await,
            wave_clear: player.load("sounds/wave_clear.wav").await,
            game_over: player.load("sounds/game_over.wav").await,
        }
    }
}

fn handle_events(game: &mut Game, player: &AudioPlayer, sounds: &Sounds, muted: bool) {
    if muted {
        return;
    }

    for event in game.take_events() {
        match event {
            GameEvent::PlayerFired => play_sound(player, &sounds.player_shot, 0.32),
            GameEvent::AlienFired => play_sound(player, &sounds.alien_shot, 0.18),
            GameEvent::AlienDestroyed => play_sound(player, &sounds.explosion, 0.42),
            GameEvent::PlayerHit => play_sound(player, &sounds.player_hit, 0.65),
            GameEvent::WaveCleared => play_sound(player, &sounds.wave_clear, 0.65),
            GameEvent::GameOver => play_sound(player, &sounds.game_over, 0.75),
        }
    }
}

fn play_sound(player: &AudioPlayer, sound: &Option<Sound>, volume: f32) {
    if let Some(sound) = sound {
        player.play(sound, volume);
    }
}

#[macroquad::main(window_conf)]
async fn main() -> AppResult<()> {
    set_pc_assets_folder(audio::ASSETS_FOLDER);

    let assets = RenderAssets::load().await?;
    let audio_player = AudioPlayer::new();
    let sounds = Sounds::load(&audio_player).await;
    let mut game = Game::new_with_seed(get_time().to_bits());
    let mut muted = false;
    let mut accumulator = 0.0;
    let mut first_frame = true;

    let render_target = render_target(WORLD_WIDTH as u32, WORLD_HEIGHT as u32);
    render_target.texture.set_filter(FilterMode::Nearest);
    let mut virtual_camera =
        Camera2D::from_display_rect(Rect::new(0.0, 0.0, WORLD_WIDTH, WORLD_HEIGHT));
    virtual_camera.render_target = Some(render_target.clone());

    loop {
        if is_key_pressed(KeyCode::Q) {
            break;
        }
        if is_key_pressed(KeyCode::M) {
            muted = !muted;
        }

        let start_pressed = is_key_pressed(KeyCode::Enter)
            || is_key_pressed(KeyCode::Space)
            || is_key_pressed(KeyCode::KpEnter);
        let pause_pressed = is_key_pressed(KeyCode::P) || is_key_pressed(KeyCode::Escape);

        match game.state {
            GameState::Ready => {
                if start_pressed {
                    game.start();
                }
            }
            GameState::Playing => {
                if pause_pressed {
                    game.toggle_pause();
                }
            }
            GameState::Paused => {
                if pause_pressed || start_pressed {
                    game.toggle_pause();
                }
            }
            GameState::GameOver => {
                if start_pressed || is_key_pressed(KeyCode::R) {
                    game.restart();
                }
            }
        }

        let input = InputState {
            move_left: is_key_down(KeyCode::Left) || is_key_down(KeyCode::A),
            move_right: is_key_down(KeyCode::Right) || is_key_down(KeyCode::D),
            fire: is_key_down(KeyCode::Space),
        };

        if game.state == GameState::Playing {
            accumulator += get_frame_time().min(0.1);
            while accumulator >= FIXED_TIME_STEP {
                game.update(FIXED_TIME_STEP, input);
                accumulator -= FIXED_TIME_STEP;
            }
        } else {
            accumulator = 0.0;
        }

        handle_events(&mut game, &audio_player, &sounds, muted);

        set_camera(&virtual_camera);
        clear_background(BLACK);
        render::draw_game(&game, &assets);

        set_default_camera();
        clear_background(BLACK);
        let scale = (screen_width() / WORLD_WIDTH).min(screen_height() / WORLD_HEIGHT);
        let destination_width = WORLD_WIDTH * scale;
        let destination_height = WORLD_HEIGHT * scale;
        let offset_x = (screen_width() - destination_width) / 2.0;
        let offset_y = (screen_height() - destination_height) / 2.0;
        draw_texture_ex(
            &render_target.texture,
            offset_x,
            offset_y,
            WHITE,
            DrawTextureParams {
                dest_size: Some(vec2(destination_width, destination_height)),
                flip_y: true,
                ..Default::default()
            },
        );

        next_frame().await;

        if first_frame {
            first_frame = false;
            notify_web_ready();
        }
    }

    Ok(())
}
