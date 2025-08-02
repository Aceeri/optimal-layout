use bevy::{color::palettes, math::U8Vec3};
use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use fxhash::FxHashMap;
use fnv::FnvHasher;
use rand::seq::SliceRandom;
use std::io::prelude::*;

use serde::{Deserialize, Serialize};

pub mod controller;
pub mod morton;

use controller::*;
use morton::*;

pub const WIDTH: usize = 16;

#[derive(Resource, Serialize, Deserialize)]
pub struct Layout(HashMap<U8Vec3, usize, fnv::FnvBuildHasher>);

pub fn linearize(point: IVec3) -> usize {
    point.x as usize + point.z as usize * WIDTH + point.y as usize * WIDTH * WIDTH
}

impl Layout {
    pub fn new_random() -> Self {
        let mut layout = Self(HashMap::with_capacity_and_hasher(WIDTH * WIDTH * WIDTH, default()));

        let mut point_list = Vec::new();
        for y in 0..WIDTH {
            for x in 0..WIDTH {
                for z in 0..WIDTH {
                    point_list.push(IVec3::new(x as i32, y as i32, z as i32));
                }
            }
        }

        let mut rng = rand::rng();
        point_list.shuffle(&mut rng);
        for (index, point) in point_list.into_iter().enumerate() {
            layout.0.insert(point.as_u8vec3(), index);
        }

        layout
    }

    pub fn new_linear() -> Self {
        let mut layout = Self(HashMap::with_capacity_and_hasher(WIDTH * WIDTH * WIDTH, default()));

        for y in 0..WIDTH {
            for x in 0..WIDTH {
                for z in 0..WIDTH {
                    let point = IVec3::new(x as i32, y as i32, z as i32);
                    layout.0.insert(point.as_u8vec3(), linearize(point));
                }
            }
        }

        layout
    }

    pub fn new_morton() -> Self {
        let mut layout = Self(HashMap::with_capacity_and_hasher(WIDTH * WIDTH * WIDTH, default()));

        for y in 0..WIDTH {
            for x in 0..WIDTH {
                for z in 0..WIDTH {
                    let point = UVec3::new(x as u32, y as u32, z as u32);
                    layout
                        .0
                        .insert(point.as_u8vec3(), to_morton_index(point) as usize);
                }
            }
        }

        layout
    }

    pub fn position(&self, point: IVec3) -> usize {
        self.0.get(&point.as_u8vec3()).copied().unwrap_or(usize::MAX)
    }

    pub fn heuristic(&self) -> usize {
        let mut total = 0;
        for (&point, &point_position) in self.0.iter() {
            for neighbor in Self::neighbors(point.as_ivec3()) {
                let neighbor_pos = self.position(neighbor);
                let distance = (neighbor_pos as isize - point_position as isize).abs() as usize;
                if distance >= 32 { // 64 bytes because we have 2 byte voxels
                    total += 1;
                }
            }
        }

        total
    }

    pub fn in_bounds(point: IVec3) -> bool {
        point.x >= 0
            && point.x < WIDTH as i32
            && point.y >= 0
            && point.y < WIDTH as i32
            && point.z >= 0
            && point.z < WIDTH as i32
    }

    pub fn neighbors(point: IVec3) -> impl Iterator<Item = IVec3> {
        (-1..=1)
            .flat_map(move |x| {
                (-1..=1).flat_map(move |y| (-1..=1).map(move |z| point + IVec3::new(x, y, z)))
            })
            // .filter(move |neighbor| *neighbor != point)
            .filter(|neighbor| Self::in_bounds(*neighbor))
    }

    pub fn swap(&mut self, a: IVec3, b: IVec3) {
        assert!(Self::in_bounds(a) && Self::in_bounds(b));
        let [a_pos, b_pos] = self.0.get_many_mut([&a.as_u8vec3(), &b.as_u8vec3()]);
        std::mem::swap(a_pos.unwrap(), b_pos.unwrap());
    }
}

use rand::{Rng, RngCore};

fn compare_bases() {
    let linear = Layout::new_linear();
    let morton = Layout::new_morton();

    let linear_heuristic = linear.heuristic();
    let morton_heuristic = morton.heuristic();
    println!(
        "linear: {:?}, morton: {:?}, {:?}%",
        linear_heuristic,
        morton_heuristic,
        ((morton_heuristic as f32 / linear_heuristic as f32) - 1.0) * 100.0,
    );
}

#[derive(Resource, Clone)]
pub struct RandomSearch {
    pub best_heuristic: usize,
    pub initial_heuristic: usize,
    pub linear_heuristic: usize,
    pub morton_heuristic: usize,

    pub per_frame: usize,
    pub iteration: usize,
    pub running: bool,
    pub load: bool,
    pub save_every: usize,

    pub run_name: &'static str,
}

impl RandomSearch {
    pub fn current_info(&self) -> String {
        format!(
            "iter: {}, best: {}, initial: {}",
            self.iteration, self.best_heuristic, self.initial_heuristic
        )
    }
}

pub fn random_search(
    mut layout: ResMut<Layout>,
    mut search: ResMut<RandomSearch>,
    input: Res<ButtonInput<KeyCode>>,
) {
    if input.just_pressed(KeyCode::KeyR) {
        search.running = !search.running;
        info!("running: {:?}", search.running);
    }

    if !search.running {
        return;
    }

    // const MAX_SWAPS: usize = WIDTH * 2.0;
    const MAX_SWAPS: usize = WIDTH * WIDTH * 3;
    // const MAX_SWAPS: usize = WIDTH * WIDTH * WIDTH;
    // const MAX_SWAPS: usize = 27;
    // const MAX_SWAPS: usize = 216;
    const PROGRESS: usize = 10_000;
    let mut rng = rand::rng();

    for _ in 0..search.per_frame {
        // try some random swaps
        if search.iteration % PROGRESS == 0 {
            println!(
                "iteration: {:?}: current best: {:?} ({:?}% initial, {:03?}% linear, {:03?}% morton)",
                search.iteration,
                search.best_heuristic,
                ((search.best_heuristic as f32 / search.initial_heuristic as f32) - 1.0)
                    * -100.0,
                ((search.best_heuristic as f32 / search.linear_heuristic as f32) - 1.0)
                    * -100.0,
                ((search.best_heuristic as f32 / search.morton_heuristic as f32) - 1.0)
                    * -100.0,
            );
        }

        const MIN: i32 = 0;
        const MAX: i32 = WIDTH as i32;
        // const MIN: i32 = 3;
        // const MAX: i32 = 6;

        let mut swaps = Vec::new();
        for _ in 0..rng.random_range(1..MAX_SWAPS) {
            let swap_a = IVec3::new(
                rng.random_range(MIN..MAX),
                rng.random_range(MIN..MAX),
                rng.random_range(MIN..MAX),
            );
            let swap_b = loop {
                let b = IVec3::new(
                    rng.random_range(MIN..MAX),
                    rng.random_range(MIN..MAX),
                    rng.random_range(MIN..MAX),
                );
                if b != swap_a {
                    break b;
                }
            };

            swaps.push((swap_a, swap_b));
        }

        // info!("swaps: {:?}", swaps);

        for (swap_a, swap_b) in swaps.iter() {
            layout.swap(*swap_a, *swap_b);
        }

        let new_heuristic = layout.heuristic();
        if new_heuristic <= search.best_heuristic {
            search.best_heuristic = new_heuristic;
        } else {
            for (swap_a, swap_b) in swaps.iter().rev() {
                layout.swap(*swap_a, *swap_b);
            }
        }

        search.iteration += 1;
    }
}

#[derive(Component, Clone)]
pub struct LayoutGizmo;

pub fn display_current_layout(
    mut commands: Commands,
    layout: Res<Layout>,
    mut gizmo_assets: ResMut<Assets<GizmoAsset>>,
    gizmo_entities: Query<Entity, With<LayoutGizmo>>,
    input: Res<ButtonInput<KeyCode>>,
) {
    if !input.just_pressed(KeyCode::KeyP) {
        return;
    }

    for entity in gizmo_entities {
        commands.entity(entity).despawn();
    }

    let mut linearized = vec![Vec3::ZERO; 16 * 16 * 16];
    for (point, index) in layout.0.iter() {
        linearized[*index] = point.as_vec3();
    }

    let mut gizmos = GizmoAsset::new();

    for window in linearized.windows(2) {
        let a = window[0];
        let b = window[1];
        gizmos.line(a, b, Color::srgb(a.x / 16.0, a.y / 16.0, a.z / 16.0));
    }

    commands.spawn((
        LayoutGizmo,
        Gizmo {
            handle: gizmo_assets.add(gizmos),
            line_config: GizmoLineConfig {
                width: 5.,
                ..default()
            },
            ..default()
        },
        Transform::from_xyz(0., 2., 0.),
    ));
}

pub fn write_layout_to_file(
    layout: Res<Layout>,
    input: Res<ButtonInput<KeyCode>>,
    search: Res<RandomSearch>,
) {
    // return;
    if !(search.iteration != 0 && search.iteration % search.save_every == 0)
        && !((input.pressed(KeyCode::ControlLeft) || input.pressed(KeyCode::ControlRight))
            && input.just_pressed(KeyCode::KeyS))
    {
        return;
    }

    info!("SAVING LAYOUT");
    let layout_buffer = serde_yml::to_string(&*layout).unwrap();

    let local_now: chrono::DateTime<chrono::Local> = chrono::Local::now();
    let now = local_now.format("%Y-%m-%d-%H:%M:%S").to_string();
    let backup_name = format!("./layouts/backup/layout-{}-{}^3-{}.yml", search.run_name, WIDTH, now);
    let name = format!("./layouts/layout-{}-{}^3.yml", search.run_name, WIDTH);
    println!("backup_name: {:?}", backup_name);
    let mut current_layout = std::fs::File::create(name).unwrap();
    let mut backup_layout = std::fs::File::create(backup_name).unwrap();
    current_layout.write_all(layout_buffer.as_bytes()).unwrap();
    backup_layout.write_all(layout_buffer.as_bytes()).unwrap();
}

pub fn load_layout_from_file(
    mut layout: ResMut<Layout>,
    mut search: ResMut<RandomSearch>,
    input: Res<ButtonInput<KeyCode>>,
) {
    // return;
    if !search.load
        && !((input.pressed(KeyCode::ControlLeft) || input.pressed(KeyCode::ControlRight))
            && input.just_pressed(KeyCode::KeyL))
    {
        return;
    }

    search.load = false;

    info!("LOADING LAYOUT");
    let name = format!("./layouts/layout-{}-{}^3.yml", search.run_name, WIDTH);
    println!("name: {:?}", name);
    let Ok(layout_str) = std::fs::read_to_string(name.clone()) else {
        warn!("No {:?} saved", name);
        return;
    };
    let deser_layout: Layout = serde_yml::from_str(&layout_str).unwrap();
    *layout = deser_layout;
    search.best_heuristic = layout.heuristic();
    search.initial_heuristic = layout.heuristic();
    search.iteration = 0;
    info!("Resetting search: {:?}", search.current_info());
}

fn main() -> AppExit {
    compare_bases();

    let mut app = App::new();
    app.add_plugins(DefaultPlugins);
    app.add_plugins(iyes_perf_ui::PerfUiPlugin);
    app.add_plugins(controller::CameraControllerPlugin);
    app.add_systems(Startup, spawn_entities);

    // let layout = Layout::new_hilbert();
    let layout = Layout::new_morton();
    // let layout = Layout::new_linear();
    // let layout = Layout::new_random();
    println!("initial heuristic: {:?}", layout.heuristic());

    app.insert_resource(RandomSearch {
        best_heuristic: layout.heuristic(),
        initial_heuristic: layout.heuristic(),
        linear_heuristic: Layout::new_linear().heuristic(),
        morton_heuristic: Layout::new_morton().heuristic(),
        per_frame: 10,
        iteration: 0,
        running: true,
        load: true,
        save_every: 1_000_000,

        run_name: "cache-morton",
    });
    app.insert_resource(layout);
    app.insert_resource(AmbientLight {
        brightness: 2500.0,
        ..default()
    });

    app.add_systems(Update, random_search);
    app.add_systems(Update, display_current_layout);

    app.add_systems(Update, (load_layout_from_file, write_layout_to_file));

    app.run()
}

pub fn spawn_entities(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.spawn(iyes_perf_ui::entries::PerfUiDefaultEntries::default());
    commands.spawn((
        // Camera {
        //     is_active: true,
        //     ..default()
        // },
        Camera3d::default(),
        CameraController::default(),
        Transform::from_xyz(5.0, 2.0, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(50.0, 50.0).subdivisions(10))),
        MeshMaterial3d(materials.add(Color::from(palettes::css::SILVER))),
    ));
}
