// TODO: use AABB instead of Rect for centered box, so collision checking doesn't have to offset by half size

use engine_simple as engine;
use engine_simple::wgpu;
use engine_simple::{geom::*, Camera, Engine, SheetRegion, Transform, Zeroable};
use rand::Rng;
use std::time::{Duration, Instant};
use std::rc::Rc;
const W: f32 = 840.0;
const H: f32 = 620.0;
const GUY_SPEED: f32 = 2.0;
const SPRITE_MAX: usize = 16;
const CATCH_DISTANCE: f32 = 16.0;
const COLLISION_STEPS: usize = 3;
struct Guy {
    pos: Vec2,
}

struct Apple {
    pos: Vec2,
    vel: Vec2,
}

struct Game {
    camera: engine::Camera,
    walls: Vec<AABB>,
    doors: Vec<AABB>,
    guy: Guy,
    guy2: Guy,
    animation: Animation, 
    animation_state: AnimationState,
    apples: Vec<Apple>,
    apple_timer: u32,
    score: u32,
    font: engine_simple::BitFont,
}
pub struct Animation {
    frames: Vec<SheetRegion>,  // Vector of frames (sprite regions)
    times: Vec<Duration>,      // Vector of times each frame should be displayed
}

impl Animation {
    // Function to sample the animation at a given time
    fn sample(&self, start_time: usize, now: usize, speedup_factor: usize) -> &SheetRegion {
        // Calculate elapsed time
        let elapsed_time = (now - start_time) * speedup_factor;

        // Determine the current frame based on elapsed time
        let mut current_frame = 0;
        let mut accumulated_time = 0;
        for (i, &frame_duration) in self.times.iter().enumerate() {
            accumulated_time += frame_duration.as_millis() as usize;
            if elapsed_time < accumulated_time {
                current_frame = i;
                break;
            }
        }

        // Return the frame to be displayed
        &self.frames[current_frame]
    }
}

// Define your AnimationState struct
pub struct AnimationState {
    current_animation: usize,
    start_time: usize,        // Starting time of the animation
    current_time: usize,      // Current time of the animation
}

// Implement methods for the AnimationState struct
impl AnimationState {
    // Function to tick the animation state
    fn tick(&mut self, now: usize) {
        self.current_time = now;
    }
}


impl engine::Game for Game {
    fn new(engine: &mut Engine) -> Self {
        let camera = Camera {
            screen_pos: [0.0, 0.0],
            screen_size: [W / 4.0, H / 4.0],
        };
        #[cfg(target_arch = "wasm32")]
        let sprite_img = {
            let img_bytes = include_bytes!("content/demo.png");
            image::load_from_memory_with_format(&img_bytes, image::ImageFormat::Png)
                .map_err(|e| e.to_string())
                .unwrap()
                .into_rgba8()
        };
        #[cfg(not(target_arch = "wasm32"))]
        let sprite_img = image::open("content/demo.png").unwrap().into_rgba8();
        let sprite_tex = engine.renderer.gpu.create_texture(
            &sprite_img,
            wgpu::TextureFormat::Rgba8UnormSrgb,
            sprite_img.dimensions(),
            Some("spr-demo.png"),
        );
        engine.renderer.sprites.add_sprite_group(
            &engine.renderer.gpu,
            &sprite_tex,
            vec![Transform::zeroed(); SPRITE_MAX], //bg, three walls, guy, a few apples
            vec![SheetRegion::zeroed(); SPRITE_MAX],
            camera,
        );
        let guy = Guy {
            pos: Vec2 {
                x: W / 2.0,
                y: 24.0,
            },
        };
        let guy2 = Guy {
            pos: Vec2 { x: 100.0, y: 200.0 },
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

        let door = AABB {
            center: Vec2 { x: 100.0, y: 200.0 },
            size: Vec2 { x: 16.0, y: 16.0 },
        };

        let door = AABB {
            center: Vec2 { x: 220.0, y: 120.0 },
            size: Vec2 { x: 16.0, y: 16.0 },
        };

        let font = engine::BitFont::with_sheet_region(
            '0'..='9',
            SheetRegion::new(0, 0, 512, 0, 80, 8),
            10,
        );
        let mut animation = Animation { 
            frames: vec![
                SheetRegion::new(0, 641, 0, 8, 13, 17)
            ], 
            times: vec![
                Duration::from_millis(100),
            ],
        };
        let mut animation_state = AnimationState { 
            current_animation: 0,
            start_time: 0, 
            current_time: 0, 
        };
        Game {
            camera,
            guy,
            guy2,
            animation,
            animation_state, 
            walls: vec![left_wall, right_wall, floor],
            doors: vec![door],
            apples: Vec::with_capacity(16),
            apple_timer: 0,
            score: 0,
            font,
        }
    }
    fn update(&mut self, engine: &mut Engine) {
        // Update camera position to follow the guy
        let guy_screen_pos = Vec2 {
            x: self.guy.pos.x - self.camera.screen_size[0] / 2.0,
            y: self.guy.pos.y - self.camera.screen_size[1] / 2.0,
        };
        self.camera.screen_pos = [guy_screen_pos.x, guy_screen_pos.y];

        let dir = engine.input.key_axis(engine::Key::Left, engine::Key::Right);
        self.guy.pos.x += dir * GUY_SPEED;
        let y_dir = engine.input.key_axis(engine::Key::Down, engine::Key::Up);
        self.guy.pos.y += y_dir * GUY_SPEED;
        let mut contacts = Vec::with_capacity(self.walls.len());
        // TODO: for multiple guys this might be better as flags on the guy for what side he's currently colliding with stuff on
        // Main Characters Hit Box
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
            // copying colission for doors
            for _iter in 0..COLLISION_STEPS {
                // player's collision box
                let guy_aabb = AABB {
                    center: self.guy.pos,
                    size: Vec2 { x: 16.0, y: 16.0 },
                };
                contacts.clear();
                contacts.extend(
                    self.doors
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
                for (door_idx, _disp) in contacts.iter() {
                    let guy_aabb = AABB {
                        center: self.guy.pos,
                        size: Vec2 { x: 16.0, y: 16.0 },
                    };

                    let door = self.doors[*door_idx];
                    // We got to a basically zero collision amount
                    // todo - 
                    // make a list of doors
                    // assign them with a list of coordinates
                    // Guy is left of wall, push left
                    // tuple to create (doorID, position)
                    
                    if self.guy.pos.x < door.center.x {
                        println!("guy position: {}",self.guy.pos.x);
                        println!("door center: {}",door.center.x);
                        self.guy.pos.x = 100.0;
                        self.guy.pos.y = 100.0;

                    }
                    // Guy is below wall, push down
                    if self.guy.pos.y < door.center.y {
                        println!("guy position: {}",self.guy.pos.x);
                        println!("door center: {}",door.center.x);
                        self.guy.pos.x = 100.0;
                        self.guy.pos.y = 100.0;
                    }
                    
                }
            }

            let mut rng = rand::thread_rng();
            if self.apple_timer > 0 {
                self.apple_timer -= 1;
            } else if self.apples.len() < 8 {
                self.apples.push(Apple {
                    pos: Vec2 {
                        x: rng.gen_range(8.0..(W - 8.0)),
                        y: H + 8.0,
                    },
                    vel: Vec2 {
                        x: 0.0,
                        y: rng.gen_range((-4.0)..(-1.0)),
                    },
                });
                self.apple_timer = rng.gen_range(30..90);
            }
            for apple in self.apples.iter_mut() {
                apple.pos += apple.vel;
            }
            if let Some(idx) = self
                .apples
                .iter()
                .position(|apple| apple.pos.distance(self.guy.pos) <= CATCH_DISTANCE)
            {
                self.apples.swap_remove(idx);
                self.score += 1;
            }
            self.apples.retain(|apple| apple.pos.y > -8.0)
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
            uvs[0] = SheetRegion::new(0, 823, 0, 16, 420, 310);

            trfs[1] = AABB {
                center: Vec2 {
                    x: W ,
                    y: H * 2.0,
                },
                size: Vec2 { x: W, y: H },
            }
            .into();
            uvs[1] = SheetRegion::new(0, 1347, 0, 16, 960, 320);
            // set walls
            const WALL_START: usize = 2;
            let guy_idx = WALL_START + self.walls.len();
            let guy2_idx = guy_idx + 1;

            for (wall, (trf, uv)) in self.walls.iter().zip(
                trfs[WALL_START..guy_idx]
                    .iter_mut()
                    .zip(uvs[WALL_START..guy_idx].iter_mut()),
            ) {
                *trf = (*wall).into();
                *uv = SheetRegion::new(0, 0, 566, 12, 1, 1);
            }
            trfs[guy2_idx] = AABB {
                center: self.guy2.pos,
                size: Vec2 { x: 16.0, y: 16.0 },
            }
            .into();
            uvs[guy2_idx] = SheetRegion::new(0, 16, 480, 8, 16, 16);
            // set guy
            trfs[guy_idx] = AABB {
                center: self.guy.pos,
                size: Vec2 { x: 13.0, y: 17.0 },
            }
            .into();
            // TODO animation frame
            uvs[guy_idx] = self.animation.frames[0];

            // check here that if down, then down animation

            // left
            if engine.input.is_key_down(engine::Key::Left) {
                uvs[guy_idx] = SheetRegion::new(0, 656, 0, 8, 13, 17);
            }
            if engine.input.is_key_down(engine::Key::Right) {
                uvs[guy_idx] = SheetRegion::new(0, 669, 0, 8, 13, 17);
            }
            if engine.input.is_key_down(engine::Key::Up) {
                uvs[guy_idx] = SheetRegion::new(0, 682, 0, 8, 13, 17);
            }

            let door_start: usize = guy_idx + 2;
            let end_of_doors: usize = door_start + self.doors.len();
            //set door
            for (door, (trf, uv)) in self.doors.iter().zip(
                trfs[door_start..end_of_doors]
                    .iter_mut()
                    .zip(uvs[door_start..end_of_doors].iter_mut()),
            ) {
                *trf = (*door).into();
                *uv = SheetRegion::new(0, 0, 566, 12, 1, 1);
            }
            trfs[guy2_idx] = AABB {
                center: self.guy2.pos,
                size: Vec2 { x: 16.0, y: 16.0 },
            }
            .into();


            // set apple
            let apple_start = guy_idx + 2 + self.doors.len();
            for (apple, (trf, uv)) in self.apples.iter().zip(
                trfs[apple_start..]
                    .iter_mut()
                    .zip(uvs[apple_start..].iter_mut()),
            ) {
                *trf = AABB {
                    center: apple.pos,
                    size: Vec2 { x: 16.0, y: 16.0 },
                }
                .into();
                *uv = SheetRegion::new(0, 0, 496, 4, 16, 16);
            }
            let sprite_count = apple_start + self.apples.len();
            let score_str = self.score.to_string();
            let text_len = score_str.len();
            engine.renderer.sprites.resize_sprite_group(
                &engine.renderer.gpu,
                0,
                sprite_count + text_len,
            );
            self.font.draw_text(
                &mut engine.renderer.sprites,
                0,
                sprite_count,
                &score_str,
                Vec2 {
                    x: 16.0,
                    y: H - 16.0,
                }
                .into(),
                16.0,
            );
            engine.renderer.sprites.upload_sprites(
                &engine.renderer.gpu,
                0,
                0..sprite_count + text_len,
            );
            engine
                .renderer
                .sprites
                .set_camera_all(&engine.renderer.gpu, self.camera);
        }
    }

fn main() {
    Engine::new(winit::window::WindowBuilder::new()).run::<Game>();
}
