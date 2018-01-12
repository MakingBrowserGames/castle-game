extern crate blit;
extern crate minifb;
extern crate line_drawing;
extern crate specs;
#[macro_use]
extern crate specs_derive;

mod draw;
mod physics;
mod terrain;

use minifb::*;
use specs::{World, DispatcherBuilder, Join};
use std::time::{SystemTime, Duration};
use std::thread::sleep;

use draw::*;
use physics::*;
use terrain::*;

const WIDTH: usize = 640;
const HEIGHT: usize = 320;

const GRAVITY: f64 = 98.1;

fn main() {
    let mut buffer: Vec<u32> = vec![0; WIDTH * HEIGHT];

    let mut world = World::new();
    world.register::<Sprite>();
    world.register::<MaskId>();
    world.register::<TerrainMask>();
    world.register::<Position>();
    world.register::<Velocity>();

    world.add_resource(Terrain::new((WIDTH, HEIGHT)));
    world.add_resource(Gravity(GRAVITY));
    world.add_resource(DeltaTime::new(1.0 / 60.0));

    let mut render = Render::new((WIDTH, HEIGHT));

    render.draw_background_from_memory(include_bytes!("../resources/sprites/background.png.blit"));
    render.draw_terrain_from_memory(&mut *world.write_resource::<Terrain>(), include_bytes!("../resources/sprites/level.png.blit"));

    let projectile = render.add_buf_from_memory(include_bytes!("../resources/sprites/projectile1.png.blit"));
    let projectile_mask = render.add_buf_from_memory(include_bytes!("../resources/masks/bighole1.png.blit"));

    let mut dispatcher = DispatcherBuilder::new()
        .add(ProjectileSystem, "projectile", &[])
        .add(SpriteSystem, "sprite", &["projectile"])
        .build();

    let options = WindowOptions {
        scale: Scale::X2,
        ..WindowOptions::default()
    };
    let mut window = Window::new("Castle Game", WIDTH, HEIGHT, options).expect("Unable to open window");

    window.set_cursor_style(CursorStyle::Crosshair);

    let mut time = SystemTime::now();
    let mut second = 0.0;
    while window.is_open() && !window.is_key_down(Key::Escape) {
        // Calculate the deltatime
        {
            let mut delta = world.write_resource::<DeltaTime>();
            *delta = DeltaTime(time.elapsed().unwrap());
            time = SystemTime::now();

            // Update the title every second
            second += delta.to_seconds();
            if second > 1.0 {
                second -= 1.0;

                let title = &format!("Castle Game - Press ESC to exit, FPS: {:.2}", 1.0 / delta.to_seconds());
                window.set_title(title);
            }
        }

        // Handle mouse events
        window.get_mouse_pos(MouseMode::Discard).map(|mouse| {
            if (second * 100.0) as i32 % 20 < 3 && window.get_mouse_down(MouseButton::Left) {
                let x = 630.0;
                let y = 190.0;
                let time = 2.0;

                let vx = ((mouse.0 as f64) - x) / time;
                let vy = (mouse.1 as f64 + 0.5 * -GRAVITY * time * time - y) / time;

                world.create_entity()
                    .with(Sprite::new(projectile))
                    .with(MaskId(projectile_mask))
                    .with(Position::new(x, y))
                    .with(Velocity::new(vx, vy))
                    .build();
            }
        });

        dispatcher.dispatch(&mut world.res);

        // Add/remove entities added in dispatch through `LazyUpdate`
        world.maintain();

        // Render the sprites & masks
        let sprites = world.read::<Sprite>();
        let terrain_masks = world.read::<TerrainMask>();
        for entity in world.entities().join() {
            if let Some(sprite) = sprites.get(entity) {
                render.draw_foreground(sprite).unwrap();
            }
            if let Some(mask) = terrain_masks.get(entity) {
                render.draw_mask_terrain(&mut *world.write_resource::<Terrain>(), mask).unwrap();

                let _ = world.entities().delete(entity);
            }
        }

        render.draw_final_buffer(&mut buffer, &*world.write_resource::<Terrain>());
        window.update_with_buffer(&buffer).unwrap();

        sleep(Duration::from_millis(1));
    }
}
