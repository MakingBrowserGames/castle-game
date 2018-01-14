use specs::*;
use blit::*;
use std::error::Error;

use physics::Position;
use terrain::*;

#[derive(Component)]
pub struct MaskId(pub usize);

#[derive(Component)]
pub struct Sprite {
    pub pos: Position,
    img_ref: usize
}

impl Sprite {
    pub fn new(img_ref: usize) -> Self {
        Sprite {
            img_ref,
            pos: Position::new(0.0, 0.0)
        }
    }

    pub fn img_ref(&self) -> usize {
        self.img_ref
    }
}

pub struct SpriteSystem;

impl<'a> System<'a> for SpriteSystem {
    type SystemData = (ReadStorage<'a, Position>,
                       WriteStorage<'a, Sprite>);

    fn run(&mut self, (pos, mut sprite): Self::SystemData) {
        for (pos, sprite) in (&pos, &mut sprite).join() {
            sprite.pos = *pos;
        }
    }
}

pub struct Render {
    background: Vec<u32>,
    foreground: Vec<u32>,

    blit_buffers: Vec<(String, BlitBuffer)>,

    width: i32,
    height: i32,
}

impl Render {
    pub fn new(size: (i32, i32)) -> Self {
        Render {
            background: vec![0; (size.0 * size.1) as usize],
            foreground: vec![0xFFFF00FF; (size.0 * size.1) as usize],

            width: size.0,
            height: size.1,

            blit_buffers: Vec::new()
        }
    }

    pub fn draw_final_buffer(&mut self, buffer: &mut Vec<u32>, terrain: &Terrain) {
        for (output, (bg, (fg, terrain))) in buffer.iter_mut().zip(self.background.iter().zip(self.foreground.iter_mut().zip(&terrain.buffer))) {
            if (*fg & 0xFFFFFF) != 0xFF00FF {
                // Draw the foreground and clear it immediately
                *output = *fg;
                *fg = 0xFF00FF;
                continue;
            }
            if (*terrain & 0xFFFFFF) != 0xFF00FF {
                // The terrain doesn't needs to be cleared
                *output = *terrain;
                continue;
            }
            *output = *bg;
        }
    }

    pub fn draw_foreground(&mut self, sprite: &Sprite) -> Result<(), Box<Error>> {
        let buf = &self.blit_buffers[sprite.img_ref()].1;

        let size = self.size();
        buf.blit(&mut self.foreground, size, sprite.pos.as_i32());

        Ok(())
    }

    pub fn draw_mask_terrain(&mut self, terrain: &mut Terrain, mask: &TerrainMask) -> Result<(), Box<Error>> {
        let buf = &self.blit_buffers[mask.id].1;

        // Center the mask
        let mut pos = mask.pos;
        pos.0 -= buf.size().0 / 2;
        pos.1 -= buf.size().1 / 2;

        let size = self.size();
        buf.blit(&mut terrain.buffer, size, pos);

        Ok(())
    }

    pub fn draw_terrain_from_memory(&mut self, terrain: &mut Terrain, bytes: &[u8]) {
        let buf = BlitBuffer::load_from_memory(bytes).unwrap();

        let size = self.size();
        buf.blit(&mut terrain.buffer, size, (0, 0));
    }

    pub fn draw_background_from_memory(&mut self, bytes: &[u8]) {
        let buf = BlitBuffer::load_from_memory(bytes).unwrap();

        let size = self.size();
        buf.blit(&mut self.background, size, (0, 0));
    }

    pub fn size(&self) -> (i32, i32) {
        (self.width, self.height)
    }

    pub fn add_buf(&mut self, name: &str, buf: BlitBuffer) -> usize {
        self.blit_buffers.push((String::from(name), buf));

        self.blit_buffers.len() - 1
    }

    pub fn add_buf_from_memory(&mut self, name: &str, bytes: &[u8]) -> usize {
        let buf = BlitBuffer::load_from_memory(bytes).unwrap();

        self.add_buf(name, buf)
    }
}
