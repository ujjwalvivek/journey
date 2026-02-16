/**--------------------------------------------------------------------------------
*!  Main game logic for Journey.
*--------------------------------------------------------------------------------**/
pub mod anim;
pub mod assets;
pub mod config;
pub mod level;
pub mod player;
use assets::KnightAnimations;
use engine::{Context, GameApp};
use level::Level;
use player::Player;
mod scene;
use scene::GameScene;

//? The main game state
//* @param player: The player character with position, velocity, and animation state
//* @param level: The current level, which handles procedural platform generation
//* @param camera_x: The horizontal offset for the camera to create a smooth follow effect
pub struct JourneyGame {
    player: Player,
    level: Level,
    camera_x: f32,
    scene: GameScene,
}

impl GameApp for JourneyGame {
    fn init(ctx: &mut Context) -> Self {
        //? Create level with infinite generation
        let level = Level::new(ctx.screen_width, ctx.screen_height);

        //? Initialize player with animations
        let animations = KnightAnimations::create_all();
        let anim_state = anim::AnimationState::new(animations, "Idle");

        //? Spawn the player with a starting position and animation state
        let start_pos = engine::Vec2::new(100.0, 1000.0);
        let player = Player::new(start_pos, anim_state);

        Self {
            player,
            level,
            camera_x: start_pos.x - ctx.screen_width / 2.0,
            scene: GameScene {
                show_collision_box: false,
                ..Default::default()
            },
        }
    }

    fn update(&mut self, ctx: &mut Context) {
        //? Update level (handles screen resize)
        self.level
            .update(self.player.position.x, ctx.screen_width, ctx.screen_height);

        //? Collect platform AABBs for collision
        let platform_aabbs: Vec<_> = self.level.platforms.iter().map(|p| p.aabb).collect();

        //? Update player with physics and input
        self.player.update(ctx, &platform_aabbs);

        //? Respawn: If player falls below -600, reset to start
        if self.player.position.y > ctx.screen_height + 500.0 {
            self.player.position = engine::Vec2::new(100.0, 1000.0);
            self.player.velocity = engine::Vec2::ZERO;
        }

        //? Clamp player to left edge only
        self.player.clamp_to_bounds(0.0, f32::INFINITY);

        //? Smooth camera follow with lerp (10% blend per frame)
        let target_camera_x = self.player.position.x - ctx.screen_width / 2.0;
        self.camera_x += (target_camera_x - self.camera_x) * 0.1;

        //? Clamp camera to prevent showing area left of x=0
        self.camera_x = self.camera_x.max(0.0);

        //? Store camera offset for renderer
        ctx.camera_offset_x = self.camera_x;
    }

    //? Render the level and player
    fn render(&mut self, ctx: &mut Context) {
        for platform in &self.level.platforms {
            let pos = platform.aabb.top_left();
            let color = level::Level::platform_color(platform.platform_type);
            ctx.draw_rect(pos, platform.aabb.size, color);
        }

        if let Some((asset_key, frame_rect)) = self
            .player
            .anim_state
            .current_frame(assets::FRAME_WIDTH, assets::FRAME_HEIGHT)
        {
            let sprite_pos = self.player.draw_position();
            let sprite_size = self.player.render_size();

            //? Map AssetKey to texture_id (1-7 for game textures)
            let texture_id = match asset_key {
                crate::anim::AssetKey::Idle => 1,
                crate::anim::AssetKey::Run => 2,
                crate::anim::AssetKey::Jump => 3,
                crate::anim::AssetKey::Fall => 4,
                crate::anim::AssetKey::Attack => 5,
                crate::anim::AssetKey::Block => 6,
                crate::anim::AssetKey::Roll => 7,
            };

            ctx.draw_sprite_from_sheet(
                sprite_pos,
                sprite_size,
                [1.0, 1.0, 1.0, 1.0],
                frame_rect,
                !self.player.facing_right,
                texture_id,
            );

            //? AABB Debug: Player's physics collision box.
            if self.scene.show_collision_box {
                let t = 2.0;
                let physics_aabb = self.player.collision_aabb();
                let phys_pos = physics_aabb.top_left();
                let phys_size = physics_aabb.size;
                ctx.draw_rect(
                    phys_pos,
                    engine::Vec2::new(phys_size.x, t),
                    [1.0, 0.0, 0.0, 1.0],
                );
                ctx.draw_rect(
                    phys_pos + engine::Vec2::new(0.0, phys_size.y - t),
                    engine::Vec2::new(phys_size.x, t),
                    [1.0, 0.0, 0.0, 1.0],
                );
                ctx.draw_rect(
                    phys_pos,
                    engine::Vec2::new(t, phys_size.y),
                    [1.0, 0.0, 0.0, 1.0],
                );
                ctx.draw_rect(
                    phys_pos + engine::Vec2::new(phys_size.x - t, 0.0),
                    engine::Vec2::new(t, phys_size.y),
                    [1.0, 0.0, 0.0, 1.0],
                );
            }
        }
    }

    fn ui(&mut self, ctx: &egui::Context, params: &mut engine::scene::SceneParams) {
        crate::scene::show_ui(ctx, &mut self.scene, params);
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
