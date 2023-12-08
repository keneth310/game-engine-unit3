// TODO: use AABB instead of Rect for centered box, so collision checking doesn't have to offset by half size

use engine_simple as engine;
use engine_simple::wgpu;
use engine_simple::{geom::*, Camera, Engine, SheetRegion, Transform, Zeroable};
use rand::Rng;
use std::thread::current;
use std::time::{Duration, Instant};
use std::rc::Rc;
const W: f32 = 840.0;
const H: f32 = 620.0;
const GUY_SPEED: f32 = 3.0;
const SPRITE_MAX: usize = 32;
const CATCH_DISTANCE: f32 = 16.0;
const COLLISION_STEPS: usize = 3;
const forest_void_x_position: f32 = 1200.0;
const forest_void_y_position: f32 = 1200.0;
const interioir_void_x_position: f32 = -300.0;
const interioir_void_y_position: f32 = 0.0;


struct Guy {
    pos: Vec2,
}

struct Apple {
    pos: Vec2,
}

struct Game {
    camera: engine::Camera,
    walls: Vec<AABB>,
    doors: Vec<Doors>,
    guy: Guy,
    guy2: Guy,
    right_animation: Animation, 
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
    fn get_current_frame(&self, animation_state: &mut AnimationState) -> &SheetRegion {
        if animation_state.start_flag{
            animation_state.start_flag = false;
            animation_state.start_time = Instant::now();
        }

        // Calculate elapsed time
        println!("Time at beginning function call: {:?}", animation_state.start_time.elapsed());
        // check if at end of list of animation frames
        println!("{}", self.frames.len()-1);
        if animation_state.current_frame > 1{
            animation_state.current_frame = 0;
            println!("YOOOOOOOO");
            animation_state.start_time = Instant::now();
        }
        println!("Animation Current Frame: {}", animation_state.current_frame);

        if animation_state.start_time.elapsed() > self.times[animation_state.current_frame]{
            animation_state.current_frame += 1;
            animation_state.start_flag = true;
        }
        // // for (i = 0, < # of frames in vec of frames)
        // for(i, &times) in self.times.iter().enumerate() {
        //     println!("Elapsed time before adding to accumulate: {}", elapsed_time);
        //     accumulated_time += elapsed_time;
        //     // if accumulated time has exceeded current duration of frame
        //     if (now.as_millis() < accumulated_time){
        //         current_frame+=1;
        //         // reset counter
        //         //accumulated_time = accumulated_time % times.as_millis();
        //         println!("Elapsed time before resetting timer: {}", elapsed_time);
        //     }
        //     else {
        //         current_frame = 0;
        //         now = Instant::now();
        //     }
        //     // loop to beginning of array of franes
        //     if (i == self.frames.len()){
        //         current_frame = 0;
        //         println!("HIT LENGTH OF FRAME{}", elapsed_time);
        //     }
        //     break;
        // }

        // // Return the frame to be displayed
        println!("return frame index: {}", animation_state.current_frame);
        &self.frames[animation_state.current_frame]
    }
}

// Define your AnimationState struct
pub struct AnimationState {
    current_animation: usize,
    current_frame: usize,      // Current time of the animation
    start_flag: bool,
    start_time: Instant,
}


impl engine::Game for Game {
    fn new(engine: &mut Engine) -> Self {
        let camera = Camera {
            screen_pos: [0.0, 0.0],
            // here to zoom
            screen_size: [W / 3.5, H / 3.5],
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
                x: -300.0,
                y: 30.0,
            },
        };
        let guy2 = Guy {
            pos: Vec2 { x: 100.0, y: 200.0 },
        };
        let floor_farm = AABB {
            center: Vec2 { x: W / 2.0, y: 108.0 },
            size: Vec2 { x: W, y: 16.0 },
        };
        let top_farm = AABB {
            center: Vec2 { x: W / 2.0, y: H - 108.0},
            size: Vec2 { x: W, y: 16.0 },
        };
        let left_farm = AABB {
            center: Vec2 { x: 108.0, y: H / 2.0 },
            size: Vec2 { x: 16.0, y: H },
        };
        let right_farm = AABB {
            center: Vec2 {
                x: W - 108.0,
                y: H / 2.0,
            },
            size: Vec2 { x: 16.0, y: H },
        };
        let floor_forest = AABB {
            center: Vec2 { x: 1200.0, y: 990.0 },
            size: Vec2 { x: 1925.0, y: 16.0 },
        };
        let top_forest = AABB {
            center: Vec2 { x: 1200.0, y: 1410.0},
            size: Vec2 { x: 1925.0, y: 16.0 },
        };
        let left_forest = AABB {
            center: Vec2 { x: 350.0, y: 1200.0 },
            size: Vec2 { x: 16.0, y: H },
        };
        let right_forest = AABB {
            center: Vec2 { x: 2055.0, y: 1200.0},
            size: Vec2 { x: 16.0, y: H },
        };
        let floor_interior = AABB {
            center: Vec2 { x: -300.0, y: -80.0 },
            size: Vec2 { x: 160.0, y: 16.0 },
        };
        let top_interior = AABB {
            center: Vec2 { x: -300.0, y: 80.0},
            size: Vec2 { x: 160.0, y: 16.0 },
        };
        let left_interior = AABB {
            center: Vec2 { x: -380.0, y: 0.0 },
            size: Vec2 { x: 16.0, y: 160.0 },
        };
        let right_interior = AABB {
            center: Vec2 { x: -220.0, y: 0.0},
            size: Vec2 { x: 16.0, y: 160.0 },
        };

        let interior_to_home = Doors {
            center: Vec2 { x: -300.0, y: -72.0 },
            size: Vec2 { x: 25.0, y: 14.0 },
            destination: Vec2{x: 435.0, y: 295.0},
        };

        let home_to_interior = Doors {
            center: Vec2 { x: 440.0, y: 320.0 },
            size: Vec2 { x: 16.0, y: 21.0 },
            destination: Vec2{x: -300.0, y: 0.0},

        };
        
        let forest_to_home = Doors {
            center: Vec2 { x: 367.0, y: 1215.0 },
            size: Vec2 { x: 16.0, y: 28.0 },
            destination: Vec2{x: 700.0, y: 232.0},

        };

        let home_to_forest = Doors {
            center: Vec2 { x: 720.0, y: 232.0 },
            size: Vec2 { x: 16.0, y: 80.0 },
            destination: Vec2{x: 390.0, y: 1215.0},
        };

        let apple1 = Apple {
            pos: Vec2 {
                x: 367.0,
                y: 1335.0,
            },
        };
        let apple2 = Apple {
            pos: Vec2 {
                x: -300.0,
                y: 0.0,
            },
        };
        let apple3 = Apple {
            pos: Vec2 {
                x: 440.0,
                y: 420.0,
            },
        };
        let apple4 = Apple {
            pos: Vec2 {
                x: 500.0,
                y: 500.0,
            },
        };
        let apple5 = Apple {
            pos: Vec2 {
                x: 2000.0,
                y: 1300.0,
            },
        };
        let apple6 = Apple {
            pos: Vec2 {
                x: 1200.0,
                y: 1200.0,
            },
        };

// font
// 012345678
// 9:;<=>?@A
// BCDEFGHIJ
// KLMNOPQRST
// UVWXYZ[\]^
// _`abcdefgh
// ijklmnopqr
// stuvwxy
// z

        // need to edit the actual spritesheet
        let font = engine::BitFont::with_sheet_region(
            '0'..='z',
            // orignal 0..9, x:0, y:512. w:80 and h:8
            // (0, 646, 77, 0, 16, 16)
            SheetRegion::new(0, 645, 75, 0, 160, 16),
            9,
        );
        let mut right_animation = Animation { 
            frames: vec![
                // right animations: 
                SheetRegion::new(0, 669, 0, 8, 13, 17),
                SheetRegion::new(0, 669, 19, 8, 12, 17), 

            ], 
            times: vec![
                Duration::from_millis(250),
                Duration::from_millis(250),
            ],
        };

        let mut animation_state = AnimationState { 
            // starts at 0
            current_animation: 0,
            current_frame: 0, 
            start_flag: true,
            start_time:Instant::now(),
        };
        Game {
            camera,
            guy,
            guy2,
            right_animation,
            animation_state, 
            walls: vec![left_farm, right_farm, floor_farm, top_farm, left_forest, right_forest, floor_forest, top_forest, floor_interior, top_interior, right_interior, left_interior], 
            doors: vec![interior_to_home, home_to_interior, forest_to_home, home_to_forest],
            apples: vec![apple1, apple2, apple3, apple4, apple5, apple6],
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
        let mut door_contacts = Vec::<Doors>::with_capacity(self.doors.len());
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

        /////////////////////////////////////////////////
        // COLLISION
            // copying collision for doors
            for _iter in 0..COLLISION_STEPS {
                let guy_aabb = AABB {
                    center: self.guy.pos,
                    size: Vec2 { x: 16.0, y: 16.0 },
                };
                // player's collision box
                let player_hitbox = Doors {
                    center: self.guy.pos,
                    size: Vec2 { x: 16.0, y: 16.0 },
                    destination: Vec2 {x: self.guy.pos[0], y: self.guy.pos[1]},
                };
                contacts.clear();
                contacts.extend(
                    // doors is a vec of doors
                    self.doors
                        .iter()
                        .enumerate()
                        .filter_map(|(ri, w)| w.displacement(player_hitbox).map(|d| (ri, d))),
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
                        self.guy.pos.x = door.destination.x;
                        self.guy.pos.y = door.destination.y;
                        // self.guy.pos.x = 430.0;
                        // self.guy.pos.y = 305.0;
                    }

                    if self.guy.pos.x > door.center.x {
                        println!("guy position: {}",self.guy.pos.x);
                        println!("door center: {}",door.center.x);
                        self.guy.pos.x = door.destination.x;
                        self.guy.pos.y = door.destination.y;
                        // self.guy.pos.x = 430.0;
                        // self.guy.pos.y = 305.0;
                    }
                    // Guy is below wall, push down
                    if self.guy.pos.y < door.center.y {
                        println!("guy position: {}",self.guy.pos.x);
                        println!("door center: {}",door.center.x);
                        self.guy.pos.x = door.destination.x;
                        self.guy.pos.y = door.destination.y;
                        // self.guy.pos.x = 430.0;
                        // self.guy.pos.y = 330.0;
                    }

                    if self.guy.pos.y > door.center.y {
                        println!("guy position: {}",self.guy.pos.x);
                        println!("door center: {}",door.center.x);
                        self.guy.pos.x = door.destination.x;
                        self.guy.pos.y = door.destination.y;
                        // self.guy.pos.x = 430.0;
                        // self.guy.pos.y = 330.0;
                    }
                    
                }
            }

            // let mut rng = rand::thread_rng();
            // if self.apple_timer > 0 {
            //     self.apple_timer -= 1;
            // } else if self.apples.len() < 8 {
            //     self.apples.push(Apple {
            //         pos: Vec2 {
            //             x: rng.gen_range(8.0..(W - 8.0)),
            //             y: H + 8.0,
            //         },
            //     });
            //     self.apple_timer = rng.gen_range(30..90);
            // }
            for apple in self.apples.iter_mut() {
            }
            /// collision with apple: 
            if let Some(idx) = self
                .apples
                .iter()
                .position(|apple| apple.pos.distance(self.guy.pos) <= CATCH_DISTANCE)
            {
                self.apples.swap_remove(idx);
                self.score += 1;
            }
            // keep apples onscreen
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
            uvs[0] = SheetRegion::new(0, 814, 0, 16, 480, 320);
            // setting forest
            // const W: f32 = 840.0;
            // const H: f32 = 620.0;
            trfs[1] = AABB {
                center: Vec2 {
                    x: 1200.0 ,
                    y: 1200.0 ,
                },
                size: Vec2 { x: 1920.0, y: H },
            }
            .into();

            uvs[1] = SheetRegion::new(0, 1316, 0, 16, 960, 320);
            // forest background ^^



            // interior room background VVV

            trfs[2] = AABB {
                center: Vec2 {
                    x: -300.0,
                    y: 0.0,
                },
                size: Vec2 { x: 160.0, y: 160.0 },
            }
            .into();

            uvs[2] = SheetRegion::new(0, 2285, 0, 16, 160, 160);
            // interior room background ^^

            // set walls
            const WALL_START: usize = 3;
            let guy_idx = WALL_START + self.walls.len();
            let guy2_idx = guy_idx + 1;

            for (wall, (trf, uv)) in self.walls.iter().zip(
                trfs[WALL_START..guy_idx]
                    .iter_mut()
                    .zip(uvs[WALL_START..guy_idx].iter_mut()),
            ) {
                *trf = (*wall).into();
                // handles the region for the walls 
        
                *uv = SheetRegion::new(0, 0, 4000, 12, 1, 1);
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
            // normal == up
            uvs[guy_idx] = SheetRegion::new(0, 641, 0, 8, 13, 17);


            // down
            if engine.input.is_key_down(engine::Key::Down) {
                uvs[guy_idx] = SheetRegion::new(0, 641, 0, 8, 13, 17);
            }

            // up
            if engine.input.is_key_down(engine::Key::Up) {
                uvs[guy_idx] = SheetRegion::new(0, 682, 0, 8, 13, 17);
            }

            
            // left
            if engine.input.is_key_down(engine::Key::Left) {
                uvs[guy_idx] = SheetRegion::new(0, 656, 0, 8, 13, 17);
            }
            // check here that if down, then down animation
            if engine.input.is_key_down(engine::Key::Right) {
                //  start keeping track of time by flipping the animation_State flag

                // 3 paramters; (start_Time, now(current time), speedup factor)
                uvs[guy_idx] = *self.right_animation.get_current_frame(&mut self.animation_state);
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
                *uv = SheetRegion::new(0, 4000, 566, 12, 1, 1);
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
                //slice - iter_mut is a method that gives each mutable element
                    .iter_mut()
                    // takes two iterators and produces new iterators with pairs of elemenets
                    .zip(uvs[apple_start..].iter_mut()),
                    //zip each apple with sheet region location
            ) {
                *trf = AABB {
                    center: apple.pos,
                    size: Vec2 { x: 16.0, y: 16.0 },
                }
                .into();
                *uv = SheetRegion::new(0, 0, 496, 4, 16, 16);
            }
            let sprite_count = apple_start + self.apples.len();
            //let score_str = self.score.to_string();
            let score_str = self.score.to_string();
            // need to update for new text - immediate engine
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
                // works for whole line
                // implement line characters, string.split
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
