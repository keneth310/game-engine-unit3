// TODO: use AABB instead of Rect for centered box, so collision checking doesn't have to offset by half size

use engine_simple as engine;
use engine_simple::wgpu;
use engine_simple::{geom::*, Camera, Engine, SheetRegion, Transform, Zeroable};
use rand::Rng;
const W: f32 = 320.0;
const H: f32 = 640.0;
const GUY_SPEED: f32 = 4.0;
const BULLET_SPEED: f32 = 48.0;
const SPRITE_MAX: usize = 32;
const CATCH_DISTANCE: f32 = 16.0;
const COLLISION_STEPS: usize = 3;
const SCROLL_SPEED: f32 = 1.0;
const accumulator: f32 = H + 8.0;
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
    vertical_screen: f32,
    scrolling_screen: f32,
    screen_buffer: bool,
    screens_loaded: f32,
    guy: Guy,
    asteroids: Vec<Asteroid>,
    bullets: Vec<Asteroid>,
    asteroid_timer: u32,
    lives: u32,
    font: engine_simple::BitFont,
    accumulator: f32,
}

impl engine::Game for Game {
    fn new(engine: &mut Engine) -> Self {
        let camera = Camera {
            // was zero
            screen_pos: [0.0, 0.0],
            //screen_pos: [-320.0, -640.0],
            // Zoom, play in W and H = 1.0
            screen_size: [W*1.0, H*1.0],
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
            center: Vec2 { x: W / 2.0 , y: 8.0 },
            size: Vec2 { x: W, y: 16.0 },
        };
        let left_wall = AABB {
            center: Vec2 { x: -160.0, y: H / 2.0 },
            size: Vec2 { x: 16.0, y: 10000.0 },
        };
        let right_wall = AABB {
            center: Vec2 {
                x: W + 160.0,
                y: H / 2.0,
            },
            size: Vec2 { x: 16.0, y: 10000.0 },
        };

        let font = engine::BitFont::with_sheet_region(
            '0'..='9',
            SheetRegion::new(0, 0, 512, 0, 80, 8),
            10,
        );
        Game {
            camera,
            guy,
            vertical_screen: 1.0,
            scrolling_screen: 1.0,
            screen_buffer: true,
            screens_loaded: 1.0,
            walls: vec![left_wall, right_wall, floor],
            asteroids: Vec::with_capacity(25),
            bullets: Vec::with_capacity(25),
            asteroid_timer: 0,
            lives: 30,
            font,
            accumulator: 0.0,
        }
    }
    fn update(&mut self, engine: &mut Engine) {
        // check if shoot button pressed
        let shooting = engine.input.is_key_pressed(engine::Key::Space);
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
        //
        // 
        //
        // timer logic for shooting
        if shooting {
            self.bullets.push(Asteroid {
                pos: Vec2 {
                    x: self.guy.pos.x,
                    y: self.guy.pos.y,
                },
                vel: Vec2 {
                    x: 0.0,
                    y: BULLET_SPEED,
                },
            });
        }
        for bullets in self.bullets.iter_mut() {
            bullets.pos += bullets.vel;
        }
        //
        //
        //
        //

        // timer logic for asteroids
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
                    x: rng.gen_range((-4.0)..(4.0)),
                    y: -25.0,
                    //y: rng.gen_range((-4.0)..(-1.0)),
                },
            });
            self.asteroid_timer = 15;
        }
        for asteroid in self.asteroids.iter_mut() {
            asteroid.pos += asteroid.vel;
        }
        // COLLISION FOR ASTEROIDS
        if let Some(idx) = self
            .asteroids
            .iter()
            .position(|asteroid| asteroid.pos.distance(self.guy.pos) <= CATCH_DISTANCE)
        {
            self.asteroids.swap_remove(idx);
            self.lives -= 1;
        }
        self.asteroids.retain(|asteroid| asteroid.pos.y > -8.0);

        // collisions for bullets

    }
    fn render(&mut self, engine: &mut Engine) {

        // scrolling camera code
        // self.camera.screen_pos[1] += 1.0;
        self.camera.screen_pos[1] += SCROLL_SPEED;
        self.guy.pos.y += SCROLL_SPEED;

        // camera following ship slowly
        // in 400 x 600 resolution, this V slows horizontal scrolling -->                         offset so you start in center
        self.camera.screen_pos[0] = (self.guy.pos.x / 2.0) - (self.camera.screen_size[0] / 2.0) + 80.0;

        // infinite background code
        //println!("{}", self.camera.screen_pos[1]);
        //println!("WERID MATH = {}", ((320.0 * self.screens_loaded) as usize));
        // once you hit the thresehold approaching end of first sreen
        if (self.screen_buffer && (self.camera.screen_pos[1] + 320.0) as usize % ((320.0 * self.screens_loaded) as usize) == 0){
            //println!("YOU JUST HIT THE SCROLLING THRESHOLD");
            self.vertical_screen += 4.0;
            self.screens_loaded += 1.0;
            self.screen_buffer = false;
        } else if (self.camera.screen_pos[1] as usize % ((640.0 * self.screens_loaded) as usize) == 0){
            self.scrolling_screen += 4.0;
            //println!("222222222222222222222222D");
            // was one when almost worked
            self.screens_loaded += 1.0;
            self.screen_buffer = true;
        }

        // SET VERTICAL SCROLLING BG IMAGES
        let (trfs, uvs) = engine.renderer.sprites.get_sprites_mut(0);
        trfs[0] = AABB {
            center: Vec2 {
                x: W / 2.0,
                y: (H / 2.0) * self.scrolling_screen,
                //y: 320.0,
            },
            size: Vec2 { x: W, y: H * 2.0 },
        }
        .into();
        uvs[0] = SheetRegion::new(0, 0, 1749, 16, 160, 640);

        trfs[1] = AABB {
            center: Vec2 {
                x: W / 2.0,
                //y: H+640.0,
                y: (H / 2.0) * self.vertical_screen,
            },
            size: Vec2 { x: W, y: H * 2.0 },
        }
        .into();
    
        uvs[1] = SheetRegion::new(0, 0, 1747, 16, 160, 640);

        // LEFT AND RIGHT WALL BG IMAGES -- JUST TWO COPIES OF ABOVE BUT SHIFTED LEFT AND RIGHT
        // // // RIGHT WALL
        trfs[2] = AABB {
            center: Vec2 {
                x: W / 2.0 + 320.0,
                y: (H / 2.0) * self.scrolling_screen,
                //y: 320.0,
            },
            size: Vec2 { x: W, y: H * 2.0 },
        }
        .into();
        uvs[2] = SheetRegion::new(0, 0, 1749, 16, 160, 640);

        trfs[3] = AABB {
            center: Vec2 {
                x: W / 2.0 + 320.0,
                //y: H+640.0,
                y: (H / 2.0) * self.vertical_screen,
            },
            size: Vec2 { x: W, y: H * 2.0 },
        }
        .into();
        uvs[3] = SheetRegion::new(0, 0, 1747, 16, 160, 640);

        
        // // // LEFT WALL
        trfs[4] = AABB {
            center: Vec2 {
                x: W / 2.0 - 320.0,
                y: (H / 2.0) * self.scrolling_screen,
                //y: 320.0,
            },
            size: Vec2 { x: W, y: H * 2.0 },
        }
        .into();
        uvs[4] = SheetRegion::new(0, 0, 1749, 16, 160, 640);

        trfs[5] = AABB {
            center: Vec2 {
                x: W / 2.0 - 320.0,
                //y: H+640.0,
                y: (H / 2.0) * self.vertical_screen,
            },
            size: Vec2 { x: W, y: H * 2.0 },
        }
        .into();
        uvs[5] = SheetRegion::new(0, 0, 1747, 16, 160, 640);
        ///////        ///////        ///////        ///////        ///////


        // set walls
        const WALL_START: usize = 7;
        let guy_idx = WALL_START + self.walls.len();
        for (wall, (trf, uv)) in self.walls.iter().zip(
            trfs[WALL_START..guy_idx]
                .iter_mut()
                .zip(uvs[WALL_START..guy_idx].iter_mut()),
        ) {
            *trf = (*wall).into();
            *uv = SheetRegion::new(0, 0, 0, 12, 8, 8);
        }
        // set guy
        trfs[guy_idx] = AABB {
            center: self.guy.pos,
            // OG JUST 26 AND 19
            size: Vec2 { x: 26.0 * 2.0, y: 19.0 * 2.0},
        }
        .into();
        // TODO animation frame
        uvs[guy_idx] = SheetRegion::new(0, 2480, 0, 8, 207, 206);

        // set bullets
        let bullet_start = guy_idx + 1;
        for (bullets, (trf, uv)) in self.bullets.iter().zip(
            trfs[bullet_start..]
                .iter_mut()
                .zip(uvs[bullet_start..].iter_mut()),
        ) {
            *trf = AABB {
                center: Vec2 { x: bullets.pos.x, y: bullets.pos.y},
                size: Vec2 { x: 16.0, y: 16.0 },
            }
            .into();
            *uv = SheetRegion::new(0, 0, 1622, 4, 16, 15);
        }

        let bullet_end = bullet_start + self.bullets.len();

        // add this to the ateroids pos val
        self.accumulator += SCROLL_SPEED;
        // set asteroids
        let asteroid_start = bullet_end;
        for (asteroid, (trf, uv)) in self.asteroids.iter().zip(
            trfs[asteroid_start..]
                .iter_mut()
                .zip(uvs[asteroid_start..].iter_mut()),
        ) {
            println!("ASTEROID acumulator: {}", self.accumulator);
            *trf = AABB {
                center: Vec2 { x: asteroid.pos.x, y: asteroid.pos.y + self.accumulator},
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
                x: 16.0 + self.camera.screen_pos[0],
                y: H - 16.0 + self.accumulator,
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
    Engine::new(winit::window::WindowBuilder::new().with_inner_size(winit::dpi::LogicalSize::new(400,600))).run::<Game>();
}
