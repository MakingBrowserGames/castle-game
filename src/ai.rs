use specs::*;
use rand;
use rand::distributions::{IndependentSample, Range};
use cgmath::MetricSpace;
use collision::Discrete;

use physics::*;
use terrain::*;
use draw::*;
use projectile::*;
use geom::*;

#[derive(Component, Debug, Copy, Clone)]
pub struct Health(pub f64);

#[derive(Component, Debug, Copy, Clone)]
pub struct Walk {
    pub bounds: BoundingBox,
    pub speed: f64
}

impl Walk {
    pub fn new(bounds: BoundingBox, speed: f64) -> Self {
        Walk { bounds, speed }
    }
}

#[derive(Component, Debug, Copy, Clone)]
pub struct Destination(pub f64);

#[derive(Component, Debug)]
pub struct Ally;

#[derive(Component, Debug)]
pub struct Enemy;

#[derive(Component, Debug)]
pub struct Turret {
    pub delay: f64,
    pub max_strength: f64,
    pub flight_time: f64,
    pub strength_variation: f64,

    delay_left: f64
}

impl Turret {
    pub fn new(delay: f64, max_strength: f64, strength_variation: f64, flight_time: f64) -> Self {
        Turret {
            delay, max_strength, flight_time, strength_variation,
            delay_left: 0.0
        }
    }
}

impl Default for Turret {
    fn default() -> Self {
        Turret {
            delay: 5.0,
            max_strength: 210.0,
            flight_time: 3.0,
            strength_variation: 10.0,

            delay_left: 0.0
        }
    }
}

#[derive(Component, Debug)]
pub struct Melee {
    dmg: f64,
    hitrate: f64,

    cooldown: f64
}

impl Melee {
    pub fn new(dmg: f64, hitrate: f64) -> Self {
        Melee {
            dmg, hitrate,

            cooldown: 0.0
        }
    }
}

pub struct WalkSystem;
impl<'a> System<'a> for WalkSystem {
    type SystemData = (Fetch<'a, DeltaTime>,
                       Fetch<'a, Terrain>,
                       ReadStorage<'a, Walk>,
                       ReadStorage<'a, Destination>,
                       WriteStorage<'a, Point>);

    fn run(&mut self, (dt, terrain, walk, dest, mut pos): Self::SystemData) {
        let dt = dt.to_seconds();

        for (walk, dest, pos) in (&walk, &dest, &mut pos).join() {
            pos.y += 1.0;

            loop {
                let hit_box = walk.bounds + *pos;
                match terrain.rect_collides(hit_box) {
                    Some(hit) => {
                        pos.y -= 1.0;

                        if hit.1 == hit_box.min.y as i32 {
                            // Top edge of bounding box is hit, don't walk anymore
                            break;
                        }

                        pos.x += walk.speed * dt * (dest.0 - pos.x).signum();
                    },
                    None => break
                }
            }
        }
    }
}

pub struct MeleeSystem;
impl<'a> System<'a> for MeleeSystem {
    type SystemData = (Entities<'a>,
                       Fetch<'a, DeltaTime>,
                       ReadStorage<'a, Ally>,
                       ReadStorage<'a, Enemy>,
                       ReadStorage<'a, Point>,
                       ReadStorage<'a, BoundingBox>,
                       WriteStorage<'a, Melee>,
                       WriteStorage<'a, Health>,
                       Fetch<'a, LazyUpdate>);

    fn run(&mut self, (entities, dt, ally, enemy, pos, bb, mut melee, mut health, updater): Self::SystemData) {
        let dt = dt.to_seconds();

        for (a, _, a_pos, a_bb) in (&*entities, &ally, &pos, &bb).join() {
            let a_aabb = *a_bb + *a_pos;
            for (e, _, e_pos, e_bb) in (&*entities, &enemy, &pos, &bb).join() {
                let e_aabb = *e_bb + *e_pos;
                if a_aabb.intersects(&*e_aabb) {
                    {
                        let a_melee: Option<&mut Melee> = melee.get_mut(a);
                        if let Some(melee) = a_melee {
                            melee.cooldown -= dt;
                            if melee.cooldown <= 0.0 {
                                reduce_unit_health(&entities, &e, health.get_mut(e).unwrap(), melee.dmg);

                                melee.cooldown = melee.hitrate;

                                let blood = entities.create();
                                updater.insert(blood, PixelParticle::new(0xFF0000, 10.0));
                                updater.insert(blood, *e_pos);
                                updater.insert(blood, Velocity::new(-10.0, -10.0));
                            }
                        }
                    }
                    {
                        let e_melee: Option<&mut Melee> = melee.get_mut(e);
                        if let Some(melee) = e_melee {
                            melee.cooldown -= dt;
                            if melee.cooldown <= 0.0 {
                                reduce_unit_health(&entities, &a, health.get_mut(a).unwrap(), melee.dmg);

                                melee.cooldown = melee.hitrate;

                                let blood = entities.create();
                                updater.insert(blood, PixelParticle::new(0xFF0000, 10.0));
                                updater.insert(blood, *a_pos);
                                updater.insert(blood, Velocity::new(-10.0, -10.0));
                            }
                        }
                    }
                }
            }
        }
    }
}

pub struct TurretSystem;
impl<'a> System<'a> for TurretSystem {
    type SystemData = (Entities<'a>,
                       Fetch<'a, DeltaTime>,
                       Fetch<'a, Gravity>,
                       ReadStorage<'a, Ally>,
                       ReadStorage<'a, Enemy>,
                       ReadStorage<'a, Point>,
                       ReadStorage<'a, Sprite>,
                       ReadStorage<'a, MaskId>,
                       ReadStorage<'a, BoundingBox>,
                       ReadStorage<'a, Damage>,
                       ReadStorage<'a, Walk>,
                       WriteStorage<'a, Turret>,
                       Fetch<'a, LazyUpdate>);

    fn run(&mut self, (entities, dt, grav, ally, enemy, pos, sprite, mask, bb, dmg, walk, mut turret, updater): Self::SystemData) {
        let dt = dt.to_seconds();
        let grav = grav.0;

        for (tpos, _, sprite, mask, bb, dmg, turret) in (&pos, &enemy, &sprite, &mask, &bb, &dmg, &mut turret).join() {
            turret.delay_left -= dt;
            if turret.delay_left > 0.0 {
                continue;
            }

            // Find the nearest ally to shoot
            let mut closest = Point::new(-1000.0, -1000.0);
            let mut dist = tpos.distance(*closest);

            for (apos, _, walk) in (&pos, &ally, &walk).join() {
                let mut pos = *apos;
                pos.x += walk.speed * turret.flight_time;

                let dist_to = tpos.distance(*pos);
                if dist_to < dist {
                    dist = dist_to;
                    closest = pos;
                }
            }

            let between = Range::new(-turret.strength_variation, turret.strength_variation);
            let mut rng = rand::thread_rng();

            let time = turret.flight_time;
            let vx = (closest.x - tpos.x) / time + between.ind_sample(&mut rng);
            let vy = (closest.y + 0.5 * -grav * time * time - tpos.y) / time + between.ind_sample(&mut rng);

            if (vx * vx + vy * vy).sqrt() < turret.max_strength {
                // Shoot the turret

                let projectile = entities.create();
                updater.insert(projectile, Point::new(tpos.x, tpos.y));
                updater.insert(projectile, Velocity::new(vx, vy));
                updater.insert(projectile, *sprite);
                updater.insert(projectile, *mask);
                updater.insert(projectile, *bb);
                updater.insert(projectile, *dmg);

                turret.delay_left = turret.delay;
            }
        }
    }
}

pub fn reduce_unit_health<'a>(entities: &'a EntitiesRes, unit: &'a Entity, health: &'a mut Health, dmg: f64) -> bool {
    health.0 -= dmg;
    if health.0 <= 0.0 {
        let _ = entities.delete(*unit);

        return true;
    }

    return false;
}
