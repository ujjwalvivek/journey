/**--------------------------------------------------------------------------------
*!  Main game logic for Journey.
*--------------------------------------------------------------------------------**/
pub mod anim;
pub mod assets;
pub mod audio;
pub mod combat;
pub mod config;
pub mod enemy;
pub mod entity;
pub mod input;
pub mod level;
pub mod level_editor;
pub mod player;
pub mod projectile;
use assets::PlayerAnimations;
use audio::{AudioAssets, AudioEvent};
use combat::moves::MoveDatabase;
use config::PhysicsConfig;
use enemy::Enemy;
use engine::egui;
use engine::{Context, FixedTime, GameApp};
use input::JourneyAction;
use level::Level;
use level_editor::LevelEditor;
use player::Player;
use projectile::ProjectilePool;
mod scene;
mod start_sequence;
use scene::GameScene;

struct VfxBurst {
    position: engine::Vec2,
    timer: u16,
    max_timer: u16,
    color: [f32; 4],
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MenuReturnState {
    StartMenu,
    Paused,
    InGame,
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum OptionsTab {
    #[default]
    Graphics,
    Physics,
    Controls,
    Audio,
}

#[derive(Debug, Clone, PartialEq)]
pub enum GameState {
    Splash {
        timer: f32,
    },
    StartMenu {
        animation_progress: f32,
    },
    Options {
        return_state: MenuReturnState,
        tab: OptionsTab,
    },
    LevelEditor {
        return_state: MenuReturnState,
    },
    InGame,
    Paused,
}

//? The main game state
//* @param player: The player character with position, velocity, and animation state
//* @param level: The current level, which handles procedural platform generation
//* @param camera_x: The horizontal offset for the camera to create a smooth follow effect
pub struct JourneyGame {
    pub(crate) player: Player,
    enemies: Vec<Enemy>,
    projectiles: ProjectilePool,
    pub(crate) level: Level,
    camera_x: f32,
    camera_y: f32,
    prev_camera_x: f32,
    prev_camera_y: f32,
    scene: GameScene,
    pub(crate) physics_config: PhysicsConfig,
    enemy_move_db: MoveDatabase,
    initial_screen_height: f32,
    screen_initialized: bool,
    init_frame_count: u32,
    cached_fps: f32,
    cached_frame_time_ms: f32,
    pending_tick_rate: u32,
    pending_target_fps: u32,
    death_respawn_timer: u32, //* 0 = Godmode, >0 = counting down to respawn
    pub(crate) level_editor: LevelEditor,
    vfx_bursts: Vec<VfxBurst>,
    using_gamepad: bool,
    pub state: GameState,
    pub show_physics_tuner_in_game: bool,
    audio_assets: AudioAssets,
    audio_music_state: AudioMusicState,
    audio_options_ducked: bool,
    pending_game_audio: Vec<AudioEvent>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AudioMusicState {
    None,
    StartScreen,
    LevelEditor,
    InGame,
}

impl JourneyGame {
    fn open_level_editor(&mut self, return_state: MenuReturnState, ctx: &Context<JourneyAction>) {
        self.state = GameState::LevelEditor { return_state };
        let start_pos = self.player.position();
        let level_floor_y = self.level.death_y_threshold - 100.0;
        self.level_editor.toggle(
            start_pos.x,
            level_floor_y,
            ctx.screen_width,
            ctx.screen_height,
        );
    }

    fn close_level_editor(&mut self, return_state: MenuReturnState) {
        self.respawn_after_level_edit();
        self.level_editor.active = false;
        self.state = match return_state {
            MenuReturnState::StartMenu => GameState::StartMenu {
                animation_progress: 1.0,
            },
            MenuReturnState::Paused => GameState::Paused,
            MenuReturnState::InGame => GameState::InGame,
        };
    }

    fn respawn_after_level_edit(&mut self) {
        self.player.entity.position = self.level.player_spawn;
        self.player.entity.velocity = engine::Vec2::ZERO;
        self.camera_x = (self.level.player_spawn.x - self.initial_screen_height / 2.0).max(0.0);
        self.camera_y = self.level.clamp_camera_y(
            self.level.player_spawn.y - self.initial_screen_height / 2.0,
            self.initial_screen_height,
        );
        self.prev_camera_x = self.camera_x;
        self.prev_camera_y = self.camera_y;
        let all_platform_aabbs: Vec<_> = self.level.platforms.iter().map(|p| p.aabb).collect();
        let mut enemies: Vec<Enemy> = self
            .level
            .enemy_spawns
            .iter()
            .map(|&(pos, etype)| Enemy::new(pos, etype))
            .collect();
        for enemy in &mut enemies {
            enemy.bind_to_platform(&all_platform_aabbs);
        }
        self.enemies = enemies;
    }

    fn update_music_state(&mut self, ctx: &mut Context<JourneyAction>) {
        let desired = match self.state {
            GameState::Splash { .. } => AudioMusicState::None,
            GameState::StartMenu { .. }
            | GameState::Options {
                return_state: MenuReturnState::StartMenu,
                ..
            } => AudioMusicState::StartScreen,
            GameState::LevelEditor { .. } => AudioMusicState::LevelEditor,
            GameState::InGame | GameState::Paused | GameState::Options { .. } => {
                AudioMusicState::InGame
            }
        };

        if desired != self.audio_music_state {
            ctx.audio.stop_loop_sfx(0.1);

            match desired {
                AudioMusicState::None => {
                    ctx.audio.stop_music(1.0);
                    ctx.audio.stop_ambience(1.0);
                }
                AudioMusicState::StartScreen => {
                    if let Some(ref data) = self.audio_assets.start_audio {
                        ctx.audio.play_music(data, 1.0);
                    }
                    ctx.audio.stop_ambience(1.0);
                }
                AudioMusicState::LevelEditor => {
                    if let Some(ref data) = self.audio_assets.ui_level_editor {
                        ctx.audio.play_music(data, 1.0);
                    }
                    ctx.audio.stop_ambience(0.5);
                }
                AudioMusicState::InGame => {
                    if let Some(ref data) = self.audio_assets.bg_music {
                        ctx.audio.play_music(data, 1.0);
                    }
                    if let Some(ref data) = self.audio_assets.ambient_audio {
                        ctx.audio.play_ambience(data, 1.0);
                    }
                }
            }
            self.audio_music_state = desired;
            self.audio_options_ducked = false;
        }

        //? WASM autoplay unlock recovery:
        //* State may already match `desired` while no loop is active. Re-attempt the desired loops.
        match desired {
            AudioMusicState::None => {}
            AudioMusicState::StartScreen => {
                if !ctx.audio.has_active_music()
                    && let Some(ref data) = self.audio_assets.start_audio
                {
                    ctx.audio.play_music(data, 0.25);
                }
            }
            AudioMusicState::LevelEditor => {
                if !ctx.audio.has_active_music()
                    && let Some(ref data) = self.audio_assets.ui_level_editor
                {
                    ctx.audio.play_music(data, 0.25);
                }
            }
            AudioMusicState::InGame => {
                if !ctx.audio.has_active_music()
                    && let Some(ref data) = self.audio_assets.bg_music
                {
                    ctx.audio.play_music(data, 0.25);
                }
                if !ctx.audio.has_active_ambience()
                    && let Some(ref data) = self.audio_assets.ambient_audio
                {
                    ctx.audio.play_ambience(data, 0.25);
                }
            }
        }

        //? Duck/unduck music when the Options menu opens or closes.
        let options_open = matches!(self.state, GameState::Options { .. });
        if options_open != self.audio_options_ducked {
            if options_open {
                let mv = ctx.audio.effective_volume(engine::AudioTrack::Music) * 0.3;
                let av = ctx.audio.effective_volume(engine::AudioTrack::Ambience) * 0.3;
                ctx.audio.set_music_live_volume(mv, 0.4);
                ctx.audio.set_ambience_live_volume(av, 0.4);
            } else {
                let mv = ctx.audio.effective_volume(engine::AudioTrack::Music);
                let av = ctx.audio.effective_volume(engine::AudioTrack::Ambience);
                ctx.audio.set_music_live_volume(mv, 0.4);
                ctx.audio.set_ambience_live_volume(av, 0.4);
            }
            self.audio_options_ducked = options_open;
        }
    }

    fn dispatch_pending_audio(&mut self, ctx: &mut Context<JourneyAction>) {
        //? Dedup and dispatch game-specific audio events
        self.pending_game_audio.sort_unstable();
        self.pending_game_audio.dedup();
        for event in self.pending_game_audio.drain(..) {
            self.audio_assets.dispatch(event, &mut ctx.audio);
        }

        //? Dispatch engine UI audio events (hover, click, checkbox, etc.)
        for ui_event in ctx.pending_ui_audio.drain(..) {
            self.audio_assets.dispatch_ui(ui_event, &mut ctx.audio);
        }
    }
}

impl GameApp for JourneyGame {
    type Action = JourneyAction;

    fn window_title() -> &'static str {
        "Journey"
    }

    fn window_icon() -> Option<&'static [u8]> {
        Some(include_bytes!("../../web/public/favicon.png"))
    }

    fn wasm_ready_event() -> Option<&'static str> {
        Some("journey:first-frame")
    }

    fn init(ctx: &mut Context<JourneyAction>) -> Self {
        input::setup_default_bindings(&mut ctx.input);
        let _tex_player = ctx.load_texture(
            include_bytes!("../assets/player/player.png"),
            "Player Spritesheet",
        );

        //? Create level with infinite generation
        let level = Level::new(ctx.screen_width, ctx.screen_height);

        //? Initialize player with animations
        let animations = PlayerAnimations::create_all();
        let anim_state = anim::AnimationState::new(animations, "Idle");

        //? Spawn player at the ASCII-defined @ tile.
        let start_pos = level.player_spawn;
        let player = Player::new(start_pos, anim_state);

        //? Spawn enemies from level data and bind each to its platform.
        let all_platform_aabbs: Vec<_> = level.platforms.iter().map(|p| p.aabb).collect();
        let mut enemies: Vec<Enemy> = level
            .enemy_spawns
            .iter()
            .map(|&(pos, etype)| Enemy::new(pos, etype))
            .collect();
        for enemy in &mut enemies {
            enemy.bind_to_platform(&all_platform_aabbs);
        }

        //? Derive initial camera position directly from spawn so there is no lerp on frame 1.
        let init_camera_x = (start_pos.x - ctx.screen_width / 2.0).max(0.0);
        let init_camera_y =
            level.clamp_camera_y(start_pos.y - ctx.screen_height / 2.0, ctx.screen_height);

        let splash_duration = if cfg!(target_arch = "wasm32") {
            0.0
        } else {
            2.0
        };

        Self {
            player,
            enemies,
            projectiles: ProjectilePool::new(),
            level,
            camera_x: init_camera_x,
            camera_y: init_camera_y,
            prev_camera_x: init_camera_x,
            prev_camera_y: init_camera_y,
            scene: GameScene {
                show_collision_box: false,
                ..Default::default()
            },
            physics_config: PhysicsConfig::default(),
            enemy_move_db: MoveDatabase::default(),
            initial_screen_height: ctx.screen_height,
            screen_initialized: false,
            init_frame_count: 0,
            cached_fps: 0.0,
            cached_frame_time_ms: 0.0,
            pending_tick_rate: ctx.fixed_tick_rate,
            pending_target_fps: ctx.target_fps,
            death_respawn_timer: 0,
            level_editor: LevelEditor::new(),
            vfx_bursts: Vec::new(),
            using_gamepad: false,
            state: GameState::Splash {
                timer: splash_duration,
            },
            show_physics_tuner_in_game: false,
            audio_assets: AudioAssets::load(),
            audio_music_state: AudioMusicState::None,
            audio_options_ducked: false,
            pending_game_audio: Vec::new(),
        }
    }

    fn fixed_update(&mut self, ctx: &mut Context<JourneyAction>, fixed_time: &FixedTime) {
        //? State Machine Handling
        match self.state {
            GameState::Splash { .. }
            | GameState::StartMenu { .. }
            | GameState::Options { .. }
            | GameState::Paused => return,
            GameState::LevelEditor { .. } | GameState::InGame => {}
        }

        //? Skip all game simulation while Level Editor is active
        if matches!(self.state, GameState::LevelEditor { .. }) {
            return;
        }

        //? Split platforms into solid and one-way for proper collision handling
        let solid_aabbs = self.level.solid_aabbs();
        let one_way_aabbs = self.level.one_way_aabbs();
        let wall_aabbs = self.level.wall_aabbs();
        let all_aabbs = self.level.all_aabbs();

        //? Update grapple target: nearest node OR staggered enemy.
        //? Staggered enemies ALWAYS take priority over static nodes (∞ range).
        //? Skip during GrapplePull/Slingshot, the target is locked when the pull begins.
        if !matches!(
            self.player.state,
            crate::player::PlayerState::GrapplePull | crate::player::PlayerState::GrappleSlingshot
        ) {
            let player_pos = self.player.position();
            let range = config::GRAPPLE_DETECT_RANGE;

            let dynamic_target = self
                .enemies
                .iter()
                .filter(|e| e.is_staggered())
                .map(|e| (e.entity.position, (e.entity.position - player_pos).length()))
                .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
                .map(|(pos, _)| pos);

            if let Some(d) = dynamic_target {
                self.player.grapple_target = Some(d);
                self.player.grapple_is_enemy_target = true;
            } else {
                self.player.grapple_target =
                    self.level.find_nearest_grapple_node(player_pos, range);
                self.player.grapple_is_enemy_target = false;
            }
        }

        self.player.fixed_update(
            ctx.delta_time,
            fixed_time.tick,
            fixed_time.tick_rate(),
            solid_aabbs,
            one_way_aabbs,
            wall_aabbs,
            &self.physics_config,
        );

        //? An enter_death() here starts the normal death -> respawn timer pipeline.
        if !self.player.is_dead && self.player.position().y > self.level.death_y_threshold {
            self.player.enter_death();
            self.death_respawn_timer = config::scale_ticks(60, fixed_time.tick_rate()) as u32;
        }

        //? Respawn player and reset all enemies + projectiles
        if self.player.is_dead {
            if self.death_respawn_timer > 0 {
                self.death_respawn_timer -= 1;
            } else {
                self.player.respawn(self.level.player_spawn);
                let all_platform_aabbs: Vec<_> =
                    self.level.platforms.iter().map(|p| p.aabb).collect();
                self.enemies = self
                    .level
                    .enemy_spawns
                    .iter()
                    .map(|&(pos, etype)| Enemy::new(pos, etype))
                    .collect();
                for enemy in &mut self.enemies {
                    enemy.bind_to_platform(&all_platform_aabbs);
                }
                self.projectiles = ProjectilePool::new();
            }

            //? Collapse prev/cur gap so interpolation_alpha can't jitter the camera while dead.
            self.prev_camera_x = self.camera_x;
            self.prev_camera_y = self.camera_y;
            return;
        }

        //? Grapple arrival at enemy: execute or bounce
        if self.player.grapple_arrived_at_enemy {
            self.player.grapple_arrived_at_enemy = false;
            let player_pos = self.player.position();

            //? Find the staggered enemy at the arrival position
            let enemy_idx = self
                .enemies
                .iter()
                .position(|e| e.is_staggered() && (e.entity.position - player_pos).length() < 24.0);

            //? Check if attack was buffered during the grapple pull
            let attack_buffered = self
                .player
                .input_buffer
                .has_attack(self.player.current_tick);

            if let Some(idx) = enemy_idx {
                if attack_buffered {
                    self.player.input_buffer.clear();
                    //? EXECUTE: attack anim → enemy dies → neon burst → Fall
                    let kill_pos = self.enemies[idx].entity.position;
                    let accent = self.enemies[idx].config.accent_color;
                    self.enemies[idx].kill();
                    self.player.entity.combat = crate::combat::CombatState::default();
                    entity::despawn_hitbox(&mut self.player.entity);
                    self.player.grapple_target = None;
                    self.player.grapple_is_enemy_target = false;
                    self.player.entity.velocity = self.player.grapple_launch_dir * 80.0;
                    self.player.state = crate::player::PlayerState::Fall;
                    self.player.anim_state.play("Fall");
                    self.player.enter_hitstop(config::scale_ticks(
                        config::HITSTOP_KILL_TICKS,
                        fixed_time.tick_rate(),
                    ));
                    ctx.trigger_freeze(8);
                    ctx.trigger_shake(6.0, 0.2);
                    //? Larger neon burst for grapple execute
                    let burst_ticks = config::scale_ticks(18, fixed_time.tick_rate());
                    self.vfx_bursts.push(VfxBurst {
                        position: kill_pos,
                        timer: burst_ticks,
                        max_timer: burst_ticks,
                        color: accent,
                    });
                } else {
                    //? BOUNCE: player bounces off the enemy like a collider
                    self.player.entity.combat = crate::combat::CombatState::default();
                    entity::despawn_hitbox(&mut self.player.entity);
                    self.player.grapple_target = None;
                    self.player.grapple_is_enemy_target = false;
                    let bounce_dir = if self.player.grapple_launch_dir.x >= 0.0 {
                        -1.0
                    } else {
                        1.0
                    };
                    self.player.entity.velocity = engine::Vec2::new(
                        bounce_dir * self.physics_config.grapple_bounce_velocity_x,
                        self.physics_config.grapple_bounce_velocity_y,
                    );
                    self.player.state = crate::player::PlayerState::Fall;
                    self.player.anim_state.play("Fall");
                }
            } else {
                //? Enemy recovered or died,then just fall
                self.player.entity.combat = crate::combat::CombatState::default();
                entity::despawn_hitbox(&mut self.player.entity);
                self.player.grapple_target = None;
                self.player.grapple_is_enemy_target = false;
                self.player.state = crate::player::PlayerState::Fall;
                self.player.anim_state.play("Fall");
            }
        }

        //? Freeze stagger timer for the enemy the player is grappling toward
        for (idx, enemy) in self.enemies.iter_mut().enumerate() {
            if !enemy.is_alive() {
                enemy.death_flash_timer = enemy.death_flash_timer.saturating_sub(1);
                continue;
            }
            //? If player is grapple-pulling to this enemy, freeze its stagger
            let is_grapple_target = self.player.grapple_is_enemy_target
                && self.player.state == crate::player::PlayerState::GrapplePull
                && self
                    .player
                    .grapple_target
                    .is_some_and(|t| (t - enemy.entity.position).length() < 4.0);
            if is_grapple_target {
                enemy.freeze_stagger();
            }

            if let Some(shoot) = enemy.fixed_update(
                ctx.delta_time,
                self.player.position(),
                all_aabbs,
                wall_aabbs,
                self.physics_config.gravity,
                self.physics_config.max_fall_speed,
                &self.enemy_move_db,
                fixed_time.tick_rate(),
            ) {
                self.projectiles.spawn(
                    shoot.origin,
                    shoot.target,
                    enemy.handle(idx),
                    shoot.speed,
                    shoot.color,
                );
                self.pending_game_audio.push(AudioEvent::Projectile);
            }
        }

        //? Hit detection: player → enemies
        for enemy in &mut self.enemies {
            if !enemy.is_alive() {
                continue;
            }
            if let Some(event) =
                entity::check_hit(&self.player.entity, &enemy.entity, &self.player.move_db)
            {
                self.player.entity.hit_landed = true;
                enemy.kill();
                let recoil_dir = if self.player.facing_right() {
                    1.0
                } else {
                    -1.0
                };
                entity::apply_knockback(&mut self.player.entity, event.recoil, recoil_dir);
                ctx.trigger_freeze(event.freeze_frames);
                ctx.trigger_shake(event.shake_intensity, 0.15);
                self.player.enter_hitstop(config::scale_ticks(
                    config::HITSTOP_KILL_TICKS,
                    fixed_time.tick_rate(),
                ));
                self.pending_game_audio.push(AudioEvent::Hit);
                break;
            }
        }

        //? Hit detection: enemies → player (1-hit kill, respects i-frames)
        if !self.player.entity.combat.invincible {
            for enemy in &mut self.enemies {
                if !enemy.is_alive() {
                    continue;
                }
                if let Some(event) =
                    entity::check_hit(&enemy.entity, &self.player.entity, &self.enemy_move_db)
                {
                    enemy.entity.hit_landed = true;
                    self.player.enter_death();
                    self.death_respawn_timer =
                        config::scale_ticks(60, fixed_time.tick_rate()) as u32;
                    let dir = if enemy.entity.facing_right { 1.0 } else { -1.0 };
                    entity::apply_knockback(&mut self.player.entity, event.knockback, dir);
                    entity::apply_knockback(&mut enemy.entity, event.recoil, dir);
                    ctx.trigger_freeze(event.freeze_frames);
                    ctx.trigger_shake(event.shake_intensity, 0.15);
                    break;
                }
            }
        }

        self.projectiles.update_all(ctx.delta_time);
        let bounce_count = self.projectiles.collide_walls(solid_aabbs, ctx.delta_time);
        for _ in 0..bounce_count {
            self.pending_game_audio.push(AudioEvent::ProjectileBounce);
        }
        if let Some(source_handle) = self.projectiles.check_parry_deflect(&self.player.entity) {
            if let Some(enemy) = source_handle.resolve_mut(&mut self.enemies) {
                enemy.enter_stagger();
            }
            ctx.trigger_freeze(5);
            ctx.trigger_shake(3.0, 0.1);
            self.pending_game_audio.push(AudioEvent::Parry);
            self.pending_game_audio.push(AudioEvent::Stagger);
        }

        if self.projectiles.check_player_hit(&self.player.entity)
            && !self.player.entity.combat.invincible
        {
            self.player.enter_death();
            self.death_respawn_timer = config::scale_ticks(60, fixed_time.tick_rate()) as u32;
        }

        //? Tick down VFX burst timers and remove expired ones
        for burst in &mut self.vfx_bursts {
            burst.timer = burst.timer.saturating_sub(1);
        }
        self.vfx_bursts.retain(|b| b.timer > 0);

        //? Single drain point: collect all player-produced audio events at end of tick.
        for ev in self.player.pending_audio.drain(..) {
            self.pending_game_audio.push(ev);
        }

        //? Deterministic camera follow (runs at fixed tick rate)
        self.prev_camera_x = self.camera_x;
        self.prev_camera_y = self.camera_y;

        let player_pos = self.player.position();
        let target_camera_x = player_pos.x - ctx.screen_width / 2.0;
        let blend = 0.1;
        self.camera_x += (target_camera_x - self.camera_x) * blend;

        let top_trigger = self.camera_y + ctx.screen_height * 0.30;
        let bottom_trigger = self.camera_y + ctx.screen_height * 0.70;
        if player_pos.y < top_trigger {
            let target_y = player_pos.y - ctx.screen_height * 0.30;
            self.camera_y += (target_y - self.camera_y) * blend;
        } else if player_pos.y > bottom_trigger {
            let target_y = player_pos.y - ctx.screen_height * 0.70;
            self.camera_y += (target_y - self.camera_y) * blend;
        }

        if (target_camera_x - self.camera_x).abs() < 0.5 {
            self.camera_x = target_camera_x;
        }

        self.camera_x = self.camera_x.max(0.0);
        self.camera_y = self.level.clamp_camera_y(self.camera_y, ctx.screen_height);
    }

    fn update(&mut self, ctx: &mut Context<JourneyAction>) {
        self.update_music_state(ctx); //* Manage music/ambience transitions based on game state
        self.dispatch_pending_audio(ctx); //* Dispatch any pending SFX events from this frame

        //? Global Input Handling (Escape, F12)
        if ctx.input.is_key_just_pressed(engine::Key::Escape) {
            match self.state {
                GameState::InGame => {
                    self.state = GameState::Paused;
                    return;
                }
                GameState::Paused => {
                    self.state = GameState::InGame;
                    return;
                }
                GameState::LevelEditor { return_state } => {
                    self.close_level_editor(return_state);
                    return;
                }
                GameState::Options { return_state, .. } => {
                    self.state = match return_state {
                        MenuReturnState::StartMenu => GameState::StartMenu {
                            animation_progress: 1.0,
                        },
                        MenuReturnState::Paused => GameState::Paused,
                        MenuReturnState::InGame => GameState::InGame,
                    };
                    return;
                }
                _ => {}
            }
        }

        if ctx.input.is_key_just_pressed(engine::Key::F12) {
            match self.state {
                GameState::InGame => {
                    self.open_level_editor(MenuReturnState::InGame, ctx);
                    return;
                }
                GameState::StartMenu { .. } => {
                    self.open_level_editor(MenuReturnState::StartMenu, ctx);
                    return;
                }
                GameState::LevelEditor { return_state } => {
                    self.close_level_editor(return_state);
                    return;
                }
                _ => {}
            }
        }

        //? State Machine Handling
        match self.state {
            GameState::Splash { ref mut timer } => {
                *timer -= ctx.delta_time;
                if *timer <= 0.0 {
                    self.state = GameState::StartMenu {
                        animation_progress: 0.0,
                    };
                }
                return;
            }
            GameState::StartMenu {
                ref mut animation_progress,
            } => {
                if *animation_progress < 1.0 {
                    *animation_progress += ctx.delta_time * 1.5;
                    if *animation_progress > 1.0 {
                        *animation_progress = 1.0;
                    }
                }
                return;
            }
            GameState::Options { .. } | GameState::Paused => return,
            GameState::LevelEditor { .. } | GameState::InGame => {}
        }

        //? track whether last input came from gamepad or keyboard/mouse
        if ctx.input.any_gamepad() {
            self.using_gamepad = true;
        } else if ctx.input.any_keyboard_or_mouse() {
            self.using_gamepad = false;
        }

        //? Cache FPS counters for UI display, sync tick rate back to engine
        self.cached_fps = ctx.fps;
        self.cached_frame_time_ms = ctx.frame_time_ms;
        ctx.fixed_tick_rate = self.pending_tick_rate;
        self.player.move_db.set_tick_rate(self.pending_tick_rate);
        self.enemy_move_db.set_tick_rate(self.pending_tick_rate);
        self.player
            .input_buffer
            .set_tick_rate(self.pending_tick_rate);
        for enemy in &mut self.enemies {
            enemy.set_tick_rate(self.pending_tick_rate);
        }
        ctx.target_fps = self.pending_target_fps;

        //? On WASM, canvas dimensions may be incorrect during initialization.
        //? Detect when screen height stabilizes and reposition player if needed.
        if !self.screen_initialized {
            self.init_frame_count += 1;

            if (ctx.screen_height - self.initial_screen_height).abs() > 10.0 {
                //? Re-anchor the level geometry to the new screen height before reading spawn.
                self.level.update(
                    self.player.position().x,
                    ctx.screen_width,
                    ctx.screen_height,
                );
                let correct_spawn = self.level.player_spawn;
                self.player.set_position(correct_spawn);
                self.camera_x = (correct_spawn.x - ctx.screen_width / 2.0).max(0.0);
                self.camera_y = self
                    .level
                    .clamp_camera_y(correct_spawn.y - ctx.screen_height / 2.0, ctx.screen_height);
                self.prev_camera_x = self.camera_x;
                self.prev_camera_y = self.camera_y;
                self.initial_screen_height = ctx.screen_height;
                self.screen_initialized = true;
            } else if self.init_frame_count > 10 {
                self.screen_initialized = true;
            }
        }

        //? Skip camera/visual updates if editing
        if matches!(self.state, GameState::LevelEditor { .. }) {
            if self.level_editor.visual_mode {
                ctx.camera_offset_x = self.level_editor.camera_x.round();
                ctx.camera_offset_y = self.level_editor.camera_y.round();
            }
            return;
        }

        //? Update level (handles screen resize) and shift entities by the same delta
        let dy = self.level.update(
            self.player.position().x,
            ctx.screen_width,
            ctx.screen_height,
        );
        if dy.abs() > 0.001 {
            self.player.entity.position.y += dy;
            self.player.shift_prev_position_y(dy);
            for enemy in &mut self.enemies {
                enemy.entity.position.y += dy;
                enemy.spawn_position.y += dy;
                if let Some(ref mut plat) = enemy.spawn_platform {
                    plat.center.y += dy;
                }
            }
            self.camera_y += dy;
            self.prev_camera_y += dy;
        }

        //? Update player (input gathering, visual state, animation, no physics)
        self.player.update(ctx);

        if !self.player.is_dead && self.player.position().y > self.level.death_y_threshold {
            self.player.respawn(self.level.player_spawn);
        }

        self.player.clamp_to_bounds(0.0, f32::INFINITY);

        //? Interpolate camera between fixed ticks for smooth rendering
        let alpha = ctx.interpolation_alpha;
        let cam_x = self.prev_camera_x + (self.camera_x - self.prev_camera_x) * alpha;
        let cam_y = self.prev_camera_y + (self.camera_y - self.prev_camera_y) * alpha;

        //? Pixel-snap camera offsets to prevent sub-pixel jitter in the renderer
        ctx.camera_offset_x = cam_x.round();
        ctx.camera_offset_y = cam_y.round();
    }

    //? Render the level and player
    fn render(&mut self, ctx: &mut Context<JourneyAction>) {
        if matches!(
            self.state,
            GameState::Splash { .. } | GameState::StartMenu { .. }
        ) {
            return;
        }

        for platform in &self.level.platforms {
            let pos = platform.aabb.top_left();
            let color = level::Level::platform_color(platform.platform_type);
            ctx.draw_rect(pos, platform.aabb.size, color);
        }

        //? Render grapple nodes with a glow
        let pulse = {
            let t = instant::SystemTime::now()
                .duration_since(instant::SystemTime::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs_f32();
            (t * 3.0).sin() * 0.5 + 0.5
        };
        for node in &self.level.grapple_nodes {
            let node_size = node.radius * 2.0;
            let base_alpha = 0.5 + pulse * 0.4;
            let in_range =
                (node.position - self.player.position()).length() <= config::GRAPPLE_DETECT_RANGE;
            let color = if in_range {
                [0.2, 0.9, 1.0, base_alpha]
            } else {
                [0.3, 0.5, 0.6, base_alpha * 0.5]
            };
            let top_left = node.position - engine::Vec2::new(node.radius, node.radius);
            ctx.draw_rect(top_left, engine::Vec2::new(node_size, node_size), color);
        }

        if let Some(frame_rect) = self.player.anim_state.current_frame(
            assets::FRAME_WIDTH,
            assets::FRAME_HEIGHT,
            assets::SHEET_COLS,
        ) {
            let sprite_pos = self.player.draw_position(ctx.interpolation_alpha);
            let sprite_size = self.player.render_size();
            let flip = !self.player.facing_right();

            ctx.draw_sprite_from_sheet(
                sprite_pos,
                sprite_size,
                [1.0, 1.0, 1.0, 1.0],
                frame_rect,
                flip,
                1,
            );

            //? Additive glow overlay during attack animations
            let is_attack = matches!(
                self.player.state,
                player::PlayerState::AttackHorizontal
                    | player::PlayerState::AttackUp
                    | player::PlayerState::AttackDown
            );
            if is_attack {
                let glow_scale = 1.08;
                let glow_size = sprite_size * glow_scale;
                let glow_offset = (glow_size - sprite_size) / 2.0;
                let glow_pos = sprite_pos - glow_offset;
                ctx.draw_sprite_from_sheet_additive(
                    glow_pos,
                    glow_size,
                    [0.6, 0.8, 1.0, 0.35],
                    frame_rect,
                    flip,
                    1,
                );
            }

            //? Additive flash during parry active frames
            if self.player.state == player::PlayerState::Parry {
                let flash_scale = 1.12;
                let flash_size = sprite_size * flash_scale;
                let flash_offset = (flash_size - sprite_size) / 2.0;
                let flash_pos = sprite_pos - flash_offset;
                ctx.draw_sprite_from_sheet_additive(
                    flash_pos,
                    flash_size,
                    [1.0, 1.0, 1.0, 0.5],
                    frame_rect,
                    flip,
                    1,
                );
            }

            //? Additive trail during dash/air-dash
            if matches!(
                self.player.state,
                player::PlayerState::Dash | player::PlayerState::AirDash
            ) {
                let trail_offset = if self.player.facing_right() {
                    -6.0
                } else {
                    6.0
                };
                let trail_pos = sprite_pos + engine::Vec2::new(trail_offset, 0.0);
                ctx.draw_sprite_from_sheet_additive(
                    trail_pos,
                    sprite_size,
                    [0.4, 0.7, 1.0, 0.25],
                    frame_rect,
                    flip,
                    1,
                );
            }

            //? Grapple line from player to target during GrapplePull
            if self.player.state == player::PlayerState::GrapplePull
                && let Some(target) = self.player.grapple_target
            {
                let player_center = self.player.position();
                let dx = target.x - player_center.x;
                let dy = target.y - player_center.y;
                let dist = (dx * dx + dy * dy).sqrt().max(1.0);
                let segments = (dist / 4.0) as usize;
                for i in 0..segments {
                    let t = i as f32 / segments as f32;
                    let px = player_center.x + dx * t;
                    let py = player_center.y + dy * t;
                    let alpha = 0.8 - t * 0.4;
                    ctx.draw_rect(
                        engine::Vec2::new(px - 1.0, py - 1.0),
                        engine::Vec2::new(2.0, 2.0),
                        [0.2, 0.9, 1.0, alpha],
                    );
                }
            }

            //? Debug overlay: color-coded boxes for player
            if self.scene.show_collision_box {
                enemy::render_debug_boxes(ctx, &self.player.entity);

                //? Wall contact indicators: small markers on touching side
                let p = self.player.position();
                let half_w = config::PLAYER_WIDTH / 2.0;
                if self.player.entity.touching_wall_left {
                    ctx.draw_rect(
                        engine::Vec2::new(p.x - half_w - 3.0, p.y - 4.0),
                        engine::Vec2::new(3.0, 8.0),
                        [1.0, 0.5, 0.0, 0.8],
                    );
                }
                if self.player.entity.touching_wall_right {
                    ctx.draw_rect(
                        engine::Vec2::new(p.x + half_w, p.y - 4.0),
                        engine::Vec2::new(3.0, 8.0),
                        [1.0, 0.5, 0.0, 0.8],
                    );
                }
            }
        }

        //? Render all alive enemies
        for enemy in &self.enemies {
            enemy::render_enemy(ctx, enemy);
        }

        //? Render projectiles
        projectile::render_projectiles(ctx, &self.projectiles);

        //? Render VFX bursts (expanding neon squares on kill)
        for burst in &self.vfx_bursts {
            let t = 1.0 - (burst.timer as f32 / burst.max_timer as f32);
            let radius = t * burst.max_timer as f32 * 2.5;
            let alpha = (1.0 - t) * 0.8;
            let ring_thickness = 2.0 + (1.0 - t) * 3.0;
            let c = burst.color;

            //? Outer glow ring
            let outer_size = engine::Vec2::new(radius * 2.0, radius * 2.0);
            let outer_pos = burst.position - engine::Vec2::new(radius, radius);
            ctx.draw_rect(outer_pos, outer_size, [c[0], c[1], c[2], alpha * 0.15]);

            //? Bright inner ring (4 rect edges)
            let inner_r = radius - ring_thickness;
            if inner_r > 0.0 {
                let ring_color = [c[0], c[1], c[2], alpha];
                let top = burst.position - engine::Vec2::new(radius, radius);
                ctx.draw_rect(
                    top,
                    engine::Vec2::new(radius * 2.0, ring_thickness),
                    ring_color,
                );
                ctx.draw_rect(
                    top + engine::Vec2::new(0.0, radius * 2.0 - ring_thickness),
                    engine::Vec2::new(radius * 2.0, ring_thickness),
                    ring_color,
                );
                ctx.draw_rect(
                    top,
                    engine::Vec2::new(ring_thickness, radius * 2.0),
                    ring_color,
                );
                ctx.draw_rect(
                    top + engine::Vec2::new(radius * 2.0 - ring_thickness, 0.0),
                    engine::Vec2::new(ring_thickness, radius * 2.0),
                    ring_color,
                );
            }

            //? Center flash (white, fades quickly)
            let flash_alpha = ((1.0 - t * 2.0).max(0.0)).powi(2);
            if flash_alpha > 0.01 {
                let flash_size = 12.0 * (1.0 - t * 0.5);
                ctx.draw_rect(
                    burst.position - engine::Vec2::new(flash_size / 2.0, flash_size / 2.0),
                    engine::Vec2::new(flash_size, flash_size),
                    [1.0, 1.0, 1.0, flash_alpha],
                );
            }
        }

        //? Debug overlay: color-coded boxes for enemies
        if self.scene.show_collision_box && self.state == GameState::InGame {
            for enemy in &self.enemies {
                if enemy.is_alive() {
                    enemy::render_debug_boxes(ctx, &enemy.entity);
                }
            }
        }
    }

    fn ui(
        &mut self,
        ctx: &egui::Context,
        engine_ctx: &mut Context<JourneyAction>,
        params: &mut engine::SceneParams,
    ) {
        match self.state {
            GameState::Splash { timer } => {
                self.show_splash_screen(ctx, timer);
            }
            GameState::StartMenu { animation_progress } => {
                self.show_start_menu(ctx, engine_ctx, animation_progress);
            }
            GameState::Options {
                return_state,
                ref tab,
            } => {
                let current_tab = *tab;
                self.show_options_menu(ctx, engine_ctx, params, return_state, current_tab);
            }
            GameState::Paused => {
                self.show_paused_menu(ctx, engine_ctx);
            }
            GameState::LevelEditor { .. } => {
                self.level_editor.show_ui(
                    ctx,
                    params,
                    &mut self.level,
                    self.initial_screen_height,
                    self.initial_screen_height,
                    &mut engine_ctx.pending_ui_audio,
                );
            }
            GameState::InGame => {
                crate::scene::show_ui(crate::scene::DebugUiParams {
                    ctx,
                    scene: &mut self.scene,
                    params,
                    fps: self.cached_fps,
                    frame_time_ms: self.cached_frame_time_ms,
                    fixed_tick_rate: &mut self.pending_tick_rate,
                    target_fps: &mut self.pending_target_fps,
                    combat: &self.player.entity.combat,
                    input_buffer: &self.player.input_buffer,
                    enemies: &self.enemies,
                    player_state: self.player.state,
                    wall_left: self.player.entity.touching_wall_left,
                    wall_right: self.player.entity.touching_wall_right,
                    dash_cooldown: self.player.dash_cooldown_timer,
                    has_air_dashed: self.player.has_air_dashed(),
                    wall_grab_timer: self.player.wall_grab_timer(),
                    grapple_target: self.player.grapple_target,
                    anim_name: self
                        .player
                        .anim_state
                        .current_animation_name()
                        .map(String::from),
                    physics_config: &mut self.physics_config,
                    using_gamepad: self.using_gamepad,
                    show_physics_tuner_in_game: self.show_physics_tuner_in_game,
                    pending_audio: &mut engine_ctx.pending_ui_audio,
                });
            }
        }
    }
}

//? WASM entry point
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(start)]
pub fn wasm_main() {
    log::info!("Target: WASM. Launching Journey Engine...");
    engine::run_wasm::<JourneyGame>();
}
