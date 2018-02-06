use specs::*;
use std::time::Duration;
use std::ops::Add;
use aabb2::{self, AABB2};

use draw::*;
use terrain::*;

#[derive(Component, Debug, Copy, Clone)]
pub struct Position {
    pub x: f64,
    pub y: f64
}

impl Position {
    pub fn new(x: f64, y: f64) -> Self {
        Position { x, y }
    }

    pub fn as_i32(&self) -> (i32, i32) {
        (self.x as i32, self.y as i32)
    }

    pub fn distance_to(&self, other: &Position) -> f64 {
        let dx = other.x - self.x;
        let dy = other.y - self.y;

        (dx * dx + dy * dy).sqrt()
    }
}

#[derive(Component, Debug, Copy, Clone)]
pub struct Velocity {
    pub x: f64,
    pub y: f64
}

impl Velocity {
    pub fn new(x: f64, y: f64) -> Self {
        Velocity { x, y }
    }
}

#[derive(Component, Debug, Copy, Clone)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64
}

impl Rect {
    pub fn new(x: f64, y: f64, width: f64, height: f64) -> Self {
        Rect { x, y, width, height }
    }

    pub fn to_i32(&self) -> (i32, i32, i32, i32) {
        (self.x as i32, self.y as i32, self.width as i32, self.height as i32)
    }
}

impl Add<Position> for Rect {
    type Output = Rect;

    fn add(self, pos: Position) -> Rect {
        Rect::new(self.x + pos.x, self.y + pos.y, self.width, self.height)
    }
}

#[derive(Component, Debug, Copy, Clone)]
pub struct BoundingBox(pub Rect);

impl BoundingBox {
    pub fn to_aabb(&self, pos: &Position) -> AABB2<f64> {
        let new_x = self.0.x + pos.x;
        let new_y = self.0.y + pos.y;
        aabb2::new([new_x, new_y],
                   [new_x + self.0.width, new_y + self.0.height])
    }
}

pub struct DeltaTime(pub Duration);

impl DeltaTime {
    pub fn new(time: f64) -> Self {
        DeltaTime(Duration::from_millis((time * 1000.0) as u64))
    }

    pub fn to_seconds(&self) -> f64 {
        self.0.as_secs() as f64 + self.0.subsec_nanos() as f64 * 1e-9
    }
}

pub struct Gravity(pub f64);

pub struct ParticleSystem;
impl<'a> System<'a> for ParticleSystem {
    type SystemData = (Entities<'a>,
                       Fetch<'a, DeltaTime>,
                       Fetch<'a, Gravity>,
                       FetchMut<'a, Terrain>,
                       WriteStorage<'a, Position>,
                       WriteStorage<'a, Velocity>,
                       WriteStorage<'a, PixelParticle>);

    fn run(&mut self, (entities, dt, grav, mut terrain, mut pos, mut vel, mut par): Self::SystemData) {
        let grav = grav.0;
        let dt = dt.to_seconds();
        
        for (entity, pos, vel, par) in (&*entities, &mut pos, &mut vel, &mut par).join() {
            pos.x += vel.x * dt;
            pos.y += vel.y * dt;
            vel.y += grav * dt;

            let old_pos = par.pos();
            match terrain.line_collides(pos.as_i32(), (old_pos.0 as i32, old_pos.1 as i32)) {
                Some(point) => {
                    terrain.draw_pixel((point.0 as usize, point.1 as usize), par.color);
                    let _ = entities.delete(entity);
                },
                None => ()
            }

            par.set_pos(pos);
            par.life -= dt;
            if par.life < 0.0 {
                let _ = entities.delete(entity);
            }
        }
    }
}
