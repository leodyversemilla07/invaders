use macroquad::prelude::*;

use crate::game::{Game, GameState, HUD_HEIGHT, WORLD_HEIGHT, WORLD_WIDTH};
use crate::highscore::HighScores;

const SPRITE_PLAYER: Rect = Rect::new(1.0, 20.0, 13.0, 8.0);
const SPRITE_PLAYER_FIRING: Rect = Rect::new(17.0, 20.0, 13.0, 8.0);
const SPRITE_ALIENS: [Rect; 4] = [
    Rect::new(50.0, 52.0, 12.0, 8.0),
    Rect::new(66.0, 52.0, 12.0, 8.0),
    Rect::new(82.0, 52.0, 12.0, 8.0),
    Rect::new(98.0, 52.0, 12.0, 8.0),
];

const COLOR_BACKGROUND_TOP: Color = Color::from_hex(0x080b1c);
const COLOR_BACKGROUND_BOTTOM: Color = Color::from_hex(0x17113a);
const COLOR_HUD: Color = Color::from_hex(0x11162f);
const COLOR_PRIMARY: Color = Color::from_hex(0xf5f7ff);
const COLOR_MUTED: Color = Color::from_hex(0x8290b5);
const COLOR_ACCENT: Color = Color::from_hex(0x64f4ff);
const COLOR_DANGER: Color = Color::from_hex(0xff4d6d);
const COLOR_BULLET: Color = Color::from_hex(0xfff07a);
const COLOR_ALIEN_BULLET: Color = Color::from_hex(0xff5577);
const COLOR_PANEL: Color = Color::from_rgba(7, 10, 28, 232);

const ALIEN_COLORS: [Color; 5] = [
    Color::from_hex(0xff4fd8),
    Color::from_hex(0x4df4ff),
    Color::from_hex(0xffe45c),
    Color::from_hex(0x71f58c),
    Color::from_hex(0xff914d),
];

pub struct RenderAssets {
    sprite_sheet: Texture2D,
    font: Font,
}

impl RenderAssets {
    pub async fn load() -> Result<Self, macroquad::Error> {
        let sprite_sheet = load_texture("sprites/alternate_space_invaders.png").await?;
        sprite_sheet.set_filter(FilterMode::Nearest);
        let font = load_ttf_font("fonts/PressStart2P-Regular.ttf").await?;
        Ok(Self { sprite_sheet, font })
    }
}

pub fn draw_game(game: &Game, assets: &RenderAssets, scores: &HighScores) {
    draw_background();
    draw_hud(game, assets);
    draw_aliens(game, assets);
    draw_bullets(game);
    draw_player(game, assets);
    draw_floor();

    match game.state {
        GameState::Ready => draw_ready_screen(assets, scores),
        GameState::Paused => draw_paused_screen(assets),
        GameState::GameOver => draw_game_over_screen(game, assets, scores),
        GameState::Playing => {
            draw_wave_clear_progress(game, assets);
            draw_wave_intro(game, assets);
        }
    }
}

fn draw_background() {
    const BANDS: usize = 12;
    let band_height = WORLD_HEIGHT / BANDS as f32;

    for band in 0..BANDS {
        let amount = band as f32 / (BANDS - 1) as f32;
        let color = Color::new(
            COLOR_BACKGROUND_TOP.r + (COLOR_BACKGROUND_BOTTOM.r - COLOR_BACKGROUND_TOP.r) * amount,
            COLOR_BACKGROUND_TOP.g + (COLOR_BACKGROUND_BOTTOM.g - COLOR_BACKGROUND_TOP.g) * amount,
            COLOR_BACKGROUND_TOP.b + (COLOR_BACKGROUND_BOTTOM.b - COLOR_BACKGROUND_TOP.b) * amount,
            1.0,
        );
        draw_rectangle(
            0.0,
            band as f32 * band_height,
            WORLD_WIDTH,
            band_height + 1.0,
            color,
        );
    }

    for star in 0..90_u32 {
        let x = ((star * 149 + 37) % 800) as f32;
        let y = HUD_HEIGHT + ((star * 83 + 19) % 510) as f32;
        let size = if star % 11 == 0 { 2.0 } else { 1.0 };
        let brightness = 0.35 + (star % 5) as f32 * 0.11;
        draw_rectangle(x, y, size, size, Color::new(0.8, 0.88, 1.0, brightness));
    }

    draw_circle(706.0, 137.0, 46.0, Color::from_rgba(113, 73, 174, 75));
    draw_circle(706.0, 137.0, 46.0, Color::from_rgba(187, 125, 255, 110));
}

fn draw_hud(game: &Game, assets: &RenderAssets) {
    draw_rectangle(0.0, 0.0, WORLD_WIDTH, HUD_HEIGHT, COLOR_HUD);
    draw_line(
        0.0,
        HUD_HEIGHT,
        WORLD_WIDTH,
        HUD_HEIGHT,
        2.0,
        Color::from_hex(0x344166),
    );

    draw_px_text(
        &format!("SCORE {:06}", game.score),
        18.0,
        21.0,
        16,
        COLOR_PRIMARY,
        &assets.font,
    );

    let wave = format!("WAVE {:02}", game.wave);
    let wave_width = measure_text(&wave, Some(&assets.font), 16, 1.0).width;
    let wave_color = if game.wave_intro_remaining() > 0.0 {
        COLOR_PRIMARY
    } else {
        COLOR_ACCENT
    };
    draw_px_text(
        &wave,
        (WORLD_WIDTH - wave_width) / 2.0,
        21.0,
        16,
        wave_color,
        &assets.font,
    );

    let ships = format!("SHIPS {}", game.lives);
    let ships_width = measure_text(&ships, Some(&assets.font), 16, 1.0).width;
    draw_px_text(
        &ships,
        WORLD_WIDTH - ships_width - 18.0,
        21.0,
        16,
        COLOR_PRIMARY,
        &assets.font,
    );
}

fn draw_aliens(game: &Game, assets: &RenderAssets) {
    let animation_frame = ((game.elapsed * 3.0).floor() as usize) % 2;

    for alien in game.aliens.iter().filter(|alien| alien.alive) {
        let pair = (alien.row % 2) * 2;
        let source = SPRITE_ALIENS[pair + animation_frame];
        let color = ALIEN_COLORS[alien.row % ALIEN_COLORS.len()];
        draw_sprite(
            &assets.sprite_sheet,
            source,
            alien.x,
            alien.y,
            alien.width,
            alien.height,
            color,
        );
    }
}

fn draw_player(game: &Game, assets: &RenderAssets) {
    let is_firing = !game.player_bullets.is_empty();
    let source = if is_firing {
        SPRITE_PLAYER_FIRING
    } else {
        SPRITE_PLAYER
    };

    let mut color = COLOR_ACCENT;
    if game.invulnerability_timer > 0.0 && (game.elapsed * 12.0).fract() < 0.5 {
        color.a = 0.3;
    }

    draw_sprite(
        &assets.sprite_sheet,
        source,
        game.player.x,
        game.player.y - 1.0,
        game.player.width,
        game.player.height + 2.0,
        color,
    );
}

fn draw_bullets(game: &Game) {
    for bullet in &game.player_bullets {
        draw_rectangle(
            bullet.x,
            bullet.y,
            bullet.width,
            bullet.height,
            COLOR_BULLET,
        );
        draw_rectangle(
            bullet.x + 1.0,
            bullet.y + bullet.height,
            bullet.width - 2.0,
            5.0,
            Color::from_rgba(255, 240, 122, 80),
        );
    }

    for bullet in &game.alien_bullets {
        draw_circle(
            bullet.x + bullet.width / 2.0,
            bullet.y + 5.0,
            4.0,
            COLOR_ALIEN_BULLET,
        );
        draw_rectangle(
            bullet.x + 1.0,
            bullet.y - 4.0,
            2.0,
            5.0,
            Color::from_rgba(255, 85, 119, 100),
        );
    }
}

fn draw_floor() {
    draw_line(
        0.0,
        WORLD_HEIGHT - 20.0,
        WORLD_WIDTH,
        WORLD_HEIGHT - 20.0,
        2.0,
        Color::from_hex(0x344166),
    );
    for x in (0..WORLD_WIDTH as i32).step_by(40) {
        draw_line(
            x as f32,
            WORLD_HEIGHT - 20.0,
            x as f32,
            WORLD_HEIGHT,
            1.0,
            Color::from_hex(0x202a4a),
        );
    }
}

fn draw_sprite(
    texture: &Texture2D,
    source: Rect,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    color: Color,
) {
    draw_texture_ex(
        texture,
        x,
        y,
        color,
        DrawTextureParams {
            source: Some(source),
            dest_size: Some(vec2(width, height)),
            ..Default::default()
        },
    );
}

fn draw_ready_screen(assets: &RenderAssets, scores: &HighScores) {
    draw_overlay(154.0, 238.0);
    draw_centered_text("INVADER PATROL", 190.0, 40, COLOR_PRIMARY, &assets.font);

    if let Some(best) = scores.best() {
        draw_centered_text(
            &format!("BEST {:06}  WAVE {:02}", best.score, best.wave),
            246.0,
            16,
            COLOR_ACCENT,
            &assets.font,
        );
    }

    draw_centered_text(
        "PRESS ENTER TO START",
        282.0,
        16,
        COLOR_ACCENT,
        &assets.font,
    );
    draw_centered_text("ARROWS OR A/D: MOVE", 322.0, 16, COLOR_MUTED, &assets.font);
    draw_centered_text(
        "SPACE: FIRE   P: PAUSE",
        352.0,
        16,
        COLOR_MUTED,
        &assets.font,
    );
}

fn draw_paused_screen(assets: &RenderAssets) {
    draw_overlay(154.0, 238.0);
    draw_centered_text("PAUSED", 260.0, 40, COLOR_PRIMARY, &assets.font);
    draw_centered_text("PRESS P TO CONTINUE", 322.0, 16, COLOR_ACCENT, &assets.font);
}

fn draw_game_over_screen(game: &Game, assets: &RenderAssets, scores: &HighScores) {
    draw_overlay(104.0, 392.0);
    draw_centered_text("GAME OVER", 140.0, 40, COLOR_DANGER, &assets.font);
    draw_centered_text(
        &format!("SCORE {:06}", game.score),
        208.0,
        20,
        COLOR_PRIMARY,
        &assets.font,
    );
    draw_centered_text(
        &format!("WAVE {:02}", game.wave),
        238.0,
        16,
        COLOR_MUTED,
        &assets.font,
    );

    if scores.recorded() {
        draw_centered_text("NEW HIGH SCORE!", 268.0, 16, COLOR_ACCENT, &assets.font);
    }

    draw_centered_text("HIGH SCORES", 306.0, 16, COLOR_MUTED, &assets.font);

    if scores.entries().is_empty() {
        draw_centered_text("NO SCORES YET", 340.0, 16, COLOR_MUTED, &assets.font);
    }

    for (index, entry) in scores.entries().iter().enumerate() {
        let text = format!("{}. {:06}  WAVE {:02}", index + 1, entry.score, entry.wave);
        draw_centered_text(
            &text,
            340.0 + index as f32 * 24.0,
            16,
            COLOR_PRIMARY,
            &assets.font,
        );
    }

    draw_centered_text(
        "PRESS ENTER TO RETRY",
        462.0,
        16,
        COLOR_ACCENT,
        &assets.font,
    );
}

fn draw_wave_clear_progress(game: &Game, assets: &RenderAssets) {
    if game.wave_clear_timer <= 0.0 {
        return;
    }

    draw_banner_panel();

    draw_centered_text("WAVE CLEAR", 250.0, 24, COLOR_ACCENT, &assets.font);
    draw_centered_text(
        &format!("+{} BONUS", game.wave_bonus()),
        292.0,
        16,
        COLOR_PRIMARY,
        &assets.font,
    );
    let width = 320.0;
    let x = (WORLD_WIDTH - width) / 2.0;
    let y = 336.0;
    draw_rectangle(x, y, width, 12.0, Color::from_rgba(255, 255, 255, 35));
    draw_rectangle(x, y, width * game.wave_progress(), 12.0, COLOR_ACCENT);
}

fn draw_wave_intro(game: &Game, assets: &RenderAssets) {
    if game.wave_intro_remaining() <= 0.0 {
        return;
    }

    draw_banner_panel();

    let remaining = game.aliens_remaining();
    draw_centered_text(
        &format!("WAVE {:02}", game.wave),
        250.0,
        40,
        COLOR_ACCENT,
        &assets.font,
    );
    draw_centered_text(
        &format!("{} INVADERS INCOMING", remaining),
        292.0,
        16,
        COLOR_PRIMARY,
        &assets.font,
    );
    draw_centered_text("GET READY", 322.0, 16, COLOR_MUTED, &assets.font);

    let width = 240.0;
    let x = (WORLD_WIDTH - width) / 2.0;
    let y = 336.0;
    draw_rectangle(x, y, width, 8.0, Color::from_rgba(255, 255, 255, 30));
    draw_rectangle(x, y, width * game.wave_intro_progress(), 8.0, COLOR_ACCENT);
}

/// A dim panel that keeps the wave banners readable over the formation.
fn draw_banner_panel() {
    draw_rectangle(120.0, 176.0, 560.0, 196.0, Color::from_rgba(7, 10, 28, 216));
    draw_rectangle_lines(120.0, 176.0, 560.0, 196.0, 2.0, Color::from_hex(0x344166));
    draw_rectangle_lines(128.0, 184.0, 544.0, 180.0, 1.0, Color::from_hex(0x1d294d));
}

fn draw_overlay(top: f32, height: f32) {
    draw_rectangle(0.0, 0.0, WORLD_WIDTH, WORLD_HEIGHT, COLOR_PANEL);
    draw_rectangle_lines(112.0, top, 576.0, height, 2.0, Color::from_hex(0x344166));
    draw_rectangle_lines(
        116.0,
        top + 4.0,
        568.0,
        height - 8.0,
        1.0,
        Color::from_hex(0x1d294d),
    );

    // A subtle decorative line keeps the panel visually connected to the HUD.
    draw_line(
        150.0,
        top + 16.0,
        650.0,
        top + 16.0,
        1.0,
        Color::from_hex(0x253258),
    );
}

fn draw_centered_text(text: &str, y: f32, size: u16, color: Color, font: &Font) {
    let width = measure_text(text, Some(font), size, 1.0).width;
    draw_px_text(text, (WORLD_WIDTH - width) / 2.0, y, size, color, font);
}

fn draw_px_text(text: &str, x: f32, y: f32, size: u16, color: Color, font: &Font) {
    draw_text_ex(
        text,
        x,
        y,
        TextParams {
            font: Some(font),
            font_size: size,
            font_scale: 1.0,
            font_scale_aspect: 1.0,
            rotation: 0.0,
            color,
        },
    );
}
