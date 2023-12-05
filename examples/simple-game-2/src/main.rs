// TODO: use AABB instead of Rect for centered box, so collision checking doesn't have to offset by half size

use engine_simple as engine;
use engine_simple::wgpu;
use engine_simple::{geom::*, Camera, Engine, SheetRegion, Transform, Zeroable};
use rand::Rng;
const W: f32 = 320.0;
const H: f32 = 640.0;
const GUY_SPEED: f32 = 4.0;
const SPRITE_MAX: usize = 32;
const CATCH_DISTANCE: f32 = 16.0;
const COLLISION_STEPS: usize = 3;
struct Guy {
    pos: Vec2,
}

struct Asteroid {
    pos: Vec2,
    vel: Vec2,
}

struct Game {
    camera: engine::Camera,
    walls: Vec<AABB>,
    guy: Guy,
    asteroids: Vec<Asteroid>,
    asteroid_timer: u32,
    lives: u32,
    font: engine_simple::BitFont,
}

impl engine::Game for Game {
    fn new(engine: &mut Engine) -> Self {
        let camera = Camera {
            screen_pos: [0.0, 0.0],
            screen_size: [W, H],
        };
        #[cfg(target_arch = "wasm32")]
        let sprite_img = {
            let img_bytes = include_bytes!("content/demo.png");
            // let img_bytes2 = include_bytes!("content/dungeon.png"); 
            image::load_from_memory_with_format(&img_bytes, image::ImageFormat::Png)
                .map_err(|e| e.to_string())
                .unwrap()
                .into_rgba8()
        };
        #[cfg(not(target_arch = "wasm32"))]
        let sprite_img = image::open("content/demo.png").unwrap().into_rgba8();
        // let sprite_img_2 = image::open("content/dungeon.png").unwrap().into_rgba8();
        // let sprite_tex = engine.renderer.gpu.create_texture(
        //     &sprite_img,
        //     wgpu::TextureFormat::Rgba8UnormSrgb,
        //     sprite_img.dimensions(),
        //     Some("spr-demo.png"),
        // );
        let sprite_tex_2 = engine.renderer.gpu.create_texture(
            &sprite_img,
            wgpu::TextureFormat::Rgba8UnormSrgb,
            sprite_img.dimensions(),
            Some("spr-demo.png"),
        );
        // engine.renderer.sprites.add_sprite_group(
        //     &engine.renderer.gpu,
        //     &sprite_tex,
        //     vec![Transform::zeroed(); SPRITE_MAX], //bg, three walls, guy, a few asteroids
        //     vec![SheetRegion::zeroed(); SPRITE_MAX],
        //     camera,
        // );
        engine.renderer.sprites.add_sprite_group(
            &engine.renderer.gpu,
            &sprite_tex_2,
            vec![Transform::zeroed(); SPRITE_MAX], //bg, three walls, guy, a few asteroids
            vec![SheetRegion::zeroed(); SPRITE_MAX],
            camera,
        );
        let guy = Guy {
            pos: Vec2 {
                x: W / 2.0,
                y: 24.0,
            },
        };
        let floor = AABB {
            center: Vec2 { x: W / 2.0, y: 8.0 },
            size: Vec2 { x: W, y: 16.0 },
        };
        let left_wall = AABB {
            center: Vec2 { x: 8.0, y: H / 2.0 },
            size: Vec2 { x: 16.0, y: H },
        };
        let right_wall = AABB {
            center: Vec2 {
                x: W - 8.0,
                y: H / 2.0,
            },
            size: Vec2 { x: 16.0, y: H },
        };

        let font = engine::BitFont::with_sheet_region(
            '0'..='9',
            SheetRegion::new(0, 0, 512, 0, 80, 8),
            10,
        );
        Game {
            camera,
            guy,
            walls: vec![left_wall, right_wall, floor],
            asteroids: Vec::with_capacity(25),
            asteroid_timer: 0,
            lives: 3,
            font,
        }
    }
    fn update(&mut self, engine: &mut Engine) {
        let dir = engine.input.key_axis(engine::Key::Left, engine::Key::Right);
        self.guy.pos.x += dir * GUY_SPEED;
        let mut contacts = Vec::with_capacity(self.walls.len());
        // TODO: for multiple guys this might be better as flags on the guy for what side he's currently colliding with stuff on
        for _iter in 0..COLLISION_STEPS {
            let guy_aabb = AABB {
                center: self.guy.pos,
                size: Vec2 { x: 16.0, y: 16.0 },
            };
            contacts.clear();
            // TODO: to generalize to multiple guys, need to iterate over guys first and have guy_index, rect_index, displacement in a contact tuple
            contacts.extend(
                self.walls
                    .iter()
                    .enumerate()
                    .filter_map(|(ri, w)| w.displacement(guy_aabb).map(|d| (ri, d))),
            );
            if contacts.is_empty() {
                break;
            }
            // This part stays mostly the same for multiple guys, except the shape of contacts is different
            contacts.sort_by(|(_r1i, d1), (_r2i, d2)| {
                d2.length_squared()
                    .partial_cmp(&d1.length_squared())
                    .unwrap()
            });
            for (wall_idx, _disp) in contacts.iter() {
                // TODO: for multiple guys should access self.guys[guy_idx].
                let guy_aabb = AABB {
                    center: self.guy.pos,
                    size: Vec2 { x: 16.0, y: 16.0 },
                };
                let wall = self.walls[*wall_idx];
                let mut disp = wall.displacement(guy_aabb).unwrap_or(Vec2::ZERO);
                // We got to a basically zero collision amount
                if disp.x.abs() < std::f32::EPSILON || disp.y.abs() < std::f32::EPSILON {
                    break;
                }
                // Guy is left of wall, push left
                if self.guy.pos.x < wall.center.x {
                    disp.x *= -1.0;
                }
                // Guy is below wall, push down
                if self.guy.pos.y < wall.center.y {
                    disp.y *= -1.0;
                }
                if disp.x.abs() <= disp.y.abs() {
                    self.guy.pos.x += disp.x;
                    // so far it seems resolved; for multiple guys this should probably set a flag on the guy
                } else if disp.y.abs() <= disp.x.abs() {
                    self.guy.pos.y += disp.y;
                    // so far it seems resolved; for multiple guys this should probably set a flag on the guy
                }
            }
        }
        let mut rng = rand::thread_rng();
        if self.asteroid_timer > 0 {
            self.asteroid_timer -= 1;
        } else if self.asteroids.len() < 25 {
            self.asteroids.push(Asteroid {
                pos: Vec2 {
                    x: rng.gen_range(8.0..(W - 8.0)),
                    y: H + 8.0,
                },
                vel: Vec2 {
                    x: 0.0,
                    y: -15.0,
                    //y: rng.gen_range((-4.0)..(-1.0)),
                },
            });
            self.asteroid_timer = rng.gen_range(30..90);
        }
        for asteroid in self.asteroids.iter_mut() {
            asteroid.pos += asteroid.vel;
        }
        if let Some(idx) = self
            .asteroids
            .iter()
            .position(|asteroid| asteroid.pos.distance(self.guy.pos) <= CATCH_DISTANCE)
        {
            self.asteroids.swap_remove(idx);
            self.lives -= 1;
        }
        self.asteroids.retain(|asteroid| asteroid.pos.y > -8.0)
    }
    fn render(&mut self, engine: &mut Engine) {
        // set bg image
        let (trfs, uvs) = engine.renderer.sprites.get_sprites_mut(0);
        trfs[0] = AABB {
            center: Vec2 {
                x: W / 2.0,
                y: H / 2.0,
            },
            size: Vec2 { x: W, y: H },
        }
        .into();
        uvs[0] = SheetRegion::new(0, 0, 1749, 16, 160, 640);
        // set walls
        const WALL_START: usize = 1;
        let guy_idx = WALL_START + self.walls.len();
        for (wall, (trf, uv)) in self.walls.iter().zip(
            trfs[WALL_START..guy_idx]
                .iter_mut()
                .zip(uvs[WALL_START..guy_idx].iter_mut()),
        ) {
            *trf = (*wall).into();
            *uv = SheetRegion::new(0, 0, 480, 12, 8, 8);
        }
        // set guy
        trfs[guy_idx] = AABB {
            center: self.guy.pos,
            size: Vec2 { x: 26.0, y: 19.0 },
        }
        .into();
        // TODO animation frame
        uvs[guy_idx] = SheetRegion::new(0, 2480, 0, 8, 207, 206);
        // set asteroids
        let asteroid_start = guy_idx + 1;
        for (asteroid, (trf, uv)) in self.asteroids.iter().zip(
            trfs[asteroid_start..]
                .iter_mut()
                .zip(uvs[asteroid_start..].iter_mut()),
        ) {
            *trf = AABB {
                center: asteroid.pos,
                size: Vec2 { x: 16.0, y: 16.0 },
            }
            .into();
            *uv = SheetRegion::new(0, 0, 1622, 4, 16, 15);
        }
        let sprite_count = asteroid_start + self.asteroids.len();
        let lives_str = self.lives.to_string();
        let text_len = lives_str.len();
        engine.renderer.sprites.resize_sprite_group(
            &engine.renderer.gpu,
            0,
            sprite_count + text_len,
        );
        self.font.draw_text(
            &mut engine.renderer.sprites,
            0,
            sprite_count,
            &lives_str,
            Vec2 {
                x: 16.0,
                y: H - 16.0,
            }
            .into(),
            16.0,
        );
        engine
            .renderer
            .sprites
            .upload_sprites(&engine.renderer.gpu, 0, 0..sprite_count + text_len);
        engine
            .renderer
            .sprites
            .set_camera_all(&engine.renderer.gpu, self.camera);
    }
}
fn main() {
    Engine::new(winit::window::WindowBuilder::new()).run::<Game>();
}
