#![allow(unused)] // silence while learning

use std::fs::File;
use std::io::prelude::*;
use std::path::Path;

// includes
use bevy::prelude::*;
//use wasm_bindgen::prelude::*;

use bevy::reflect::TypeRegistry;
use bevy::asset::LoadState;
use bevy::render::camera::OrthographicProjection;
use bevy::sprite::TextureAtlasBuilder;
// inputs
use bevy::input::{ ElementState, keyboard::KeyCode, keyboard::KeyboardInput, Input};
// diagnostics
use bevy::diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin};
// physics and collision API
//use heron::prelude::*;
use rand::prelude::*;
use std::collections::HashMap;
//#![feature(duration_constants)]
use std::time::Duration;

use heron::prelude::*;
use heron::RigidBody as RigidBodyEnum;

use heron::rapier_plugin::rapier2d::dynamics::RigidBody as RigidBodyStruct;
use heron::rapier_plugin::rapier2d::dynamics::RigidBodyBuilder;
////////////////////////////////
// Global consts/vars start //
////////////////////////////////

// how large loaded tiles in pixels are (can change)
const TILE_ACTUALSIZE: f32 = 16.0 as f32;
// how large should the tiles be in pixels (width and height)
const TILE_GOALSIZE: f32 = 12.0 as f32;

// parameter used in translation, where half-extend of tile size.
const TILE_UNIT_TRANSLATION : f32 = TILE_GOALSIZE / 2.0;

const TILES_X: u32 = 48;
const TILES_Y: u32 = 40;

//const TILES_X: u32 = 96;
//const TILES_Y: u32 = 80;

// TODO: need to make WIN SIZE constant (ratio of TILE_GOALSIZE to number of tiles with given win size.)
// 24 tiles of width
const WIN_WIDTH : f32 = (TILE_GOALSIZE * (TILES_X as f32) );
// 20 tiles of height
const WIN_HEIGHT : f32 = (TILE_GOALSIZE * (TILES_Y as f32) );

// Scaling of player sprite
const PLAYER_SCALE_X : f32 = 0.15 as f32;
const PLAYER_SCALE_Y : f32 = 0.15 as f32;

const MOVE_STEPS : u32 = 4;

// how far do sprites move each step.
const MOVE_DISTANCE_X : f32 = TILE_GOALSIZE;
const MOVE_DISTANCE_Y : f32 = TILE_GOALSIZE;
const MOVE_DISTANCE_XY : f32 = TILE_GOALSIZE * 0.75; // should be ca. 0.701 (unit circle 45*)
                                                        // but nothing is perfect ...
/*
    - MOVE STEP happens every frame or two. (to make movement look fluid)
    one keydown amounts to MOVE_DISTANCE_X/MOVE_DISTANCE_Y/MOVE_DISTANCE_XY in general
    - since diagonal movement interferes with the grid system (if its proportional to horizontal/vertical movement)
    the system checks if boundaries breached every frame and
    makes the total MOVE_DISTANCE shorter if collision happens.
*/

const MOVE_STEP_X : f32 = MOVE_DISTANCE_X / MOVE_STEPS as f32;
const MOVE_STEP_Y : f32 = MOVE_DISTANCE_Y / MOVE_STEPS as f32;
const MOVE_STEP_XY : f32 = MOVE_DISTANCE_XY / MOVE_STEPS as f32;

//const SPRITE_ROAD: &str = "road01.png";
const SPRITE_BACKGROUND : &str = "background-rpg01.png";

const TIME_STEP: f32 = 1. / 60.;

///////////////////////////////////
// global vars accessed externally
///////////////////////////////////

// variable accessed by javascript keydown eventhandler continuously
// to check if and what game object has been 'accessed' by player.

#[cfg(target_arch = "wasm32")]
static mut PAGE_ID : GateIdentifier = GateIdentifier::None;

static WAIT_TIME: u64 = 2;


#[derive(Copy, Clone)]
enum ExtGameObjectId {
    None = 3000,
    Home = 3001,
    Blog = 3002,
    Markets = 3003,
    Portfolio = 3004,
    Contact = 3005 // rename to 'About'
}

enum MovementDir {
    MoveLeft,
    MoveRight,
    MoveUp,
    MoveDown,

    // diagonal movement 
    // (special case to prevent clunkiness on the screen)
    MovementUpLeft,
    MovementUpRight,
    MovementDownLeft,
    MovementDownRight
}

#[cfg(target_arch = "wasm32")]
#[derive(PartialEq, Eq, Hash, Copy, Clone)]
enum GateIdentifier {
    None = 3000,
    Home = 3001,
    Blog = 3002,
    Markets = 3003,
    About = 3004
}

#[derive(PartialEq, Eq, Hash, Copy, Clone)]
enum AnimState {
    Idle = 4000,
    Run = 4001,
    Attack = 4002,
    Jump = 4003,
    Glide = 4004
}

#[derive(Default)]
struct AnimStateTuple {
    old: Option<AnimState>,
    current: Option<AnimState>
}


#[derive(PartialEq, Eq, Hash, Copy, Clone)]
enum TileType {
    Path,
    Grass,
    GrassBottom,
    GrassTop,
    GrassRight,
    GrassLeft,
    GrassTopleft,
    GrassTopright,
    GrassBottomleft,
    GrassBottomright,
}
//////////////////////////////
// Global consts/vars end 
//////////////////////////////

//////////////////////
// Resources start //
//////////////////////
/// 
/// 

#[derive(Default)]
struct Timers {
    gate_timer: Timer,
    movement_timer: Timer,

    anim_run_timer: Timer, // start once player presses jump. only attacks can interrupt the jump. timer property of playerbundle sets the animation speed.
//    anim_idle_timer: Timer, // 
    anim_attack_timer: Timer, // start once player presses attack. only jumps can interrupt the attack. timer property of playerbundle sets the animation speed.
    anim_jump_timer: Timer
}

#[derive(Default)]
struct AtlasHandles {
    player: HashMap<AnimState, Handle<TextureAtlas>>,
    // ...
    idle: Handle<TextureAtlas>,
    run: Handle<TextureAtlas>,
    attack: Handle<TextureAtlas>,
    background: Handle<TextureAtlas>
}

// holding tl and br points of all instantiated objects(excluding player)
#[cfg(target_arch = "wasm32")]
#[derive(Default)]
struct Points {                    //topleft, bottomright
    hash_map: HashMap<GateIdentifier, (Vec3, Vec3)>,
}

#[derive(Default)]
struct ActionDesc {
    vel_step: Vec3, // step in which speed increases
    vel_max: Vec3, // max speed.

    acc_step: Vec3, // step in which acceleration increases.
    acc_max: Vec3, // max acceleration

    move_dir: Option<MovementDir>
}

struct BoxTexture {
    h_texture: Handle<Texture>
}

#[derive(Default)]
struct SpriteHandles {
    // unit tiles
    player_new: HashMap<AnimState, Vec<HandleUntyped>>,
//    player: Vec<HandleUntyped>, //consider making a hashmap, key: enum (Idle, Running, etc.) value: Vec<HandleUnTyped>
    opponent: Vec<HandleUntyped>,
    // ground tiles (16px width, 16px height)
    grass: Vec<HandleUntyped>,
    // put grass, path, etc into here.
    tiles: HashMap<TileType, Vec<HandleUntyped>>,

    path: Vec<HandleUntyped>,
    grass_bottom: Vec<HandleUntyped>,
    grass_top: Vec<HandleUntyped>,
    grass_right: Vec<HandleUntyped>,
    grass_left: Vec<HandleUntyped>,
    grass_topleft: Vec<HandleUntyped>,
    grass_topright: Vec<HandleUntyped>,
    grass_bottomleft: Vec<HandleUntyped>,
    grass_bottomright: Vec<HandleUntyped>,

    home: Handle<Texture>,

    background: Option<HandleUntyped>
}

//#[derive(Debug)]
struct Materials {
    player_materials: Handle<ColorMaterial>,
}

//#[derive(Debug)]
struct WinSize {
    w:      f32,
    h:      f32,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum AppState {
    Load,
    Setup,
    Ready
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[derive(SystemLabel)]
enum AppSystems {
    ResourceLoading,
    ResourceInit,
    GuiInit,
    SpriteInit,
    Animation
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[derive(StageLabel)]
enum AppStages {
    Load,
    Setup,
    Finished,
    Run
}

/*
// Define your physics layers (Heron)
#[derive(PhysicsLayer)]
enum Layer {
    World,
    Player,
    Enemies,
}
*/

////////////////////
// Resources end //
////////////////////

//////////////////////
// Components start
//////////////////////


#[derive(Reflect)]
#[reflect(Component)]
struct ComponentB {
    pub value: String,
    #[reflect(ignore)]
    pub time_since_startup: std::time::Duration,
}

impl FromWorld for ComponentB {
    fn from_world(world: &mut World) -> Self {
        let time = world.get_resource::<Time>().unwrap();
        ComponentB {
            time_since_startup: time.time_since_startup(),
            value: "Default Value".to_string(),
        }
    }
}

#[derive(Copy, Clone)]
struct TexSize {
    w: f32,
    h: f32,
    scale_w: f32,
    scale_h: f32
}

struct IsPlayer(bool);
struct IsDestructible(bool);
struct IsStatic(bool);

#[derive(Debug)]
struct Health(f32);

#[derive(Debug)]
struct AttackPoints(f32);

enum Allegiance {
    Blue=     1700,
    Red=      1701,
    Yellow=   1702,
    None=     1703
}

// Marker Components (empty structs, useful with query filters)
#[derive(Debug)]
struct Camera2d;
#[derive(Debug)]
struct CameraUi;
#[derive(Debug)]
struct InGameUi;
#[derive(Debug)]
struct MainMenuUi;
#[derive(Debug)]
struct Player;
#[derive(Debug)]
struct Creature;
#[derive(Debug)]
struct Structure;
#[derive(Debug)]
struct Boss;

// Market components for rendered objects linking to websites pages.
#[derive(Debug)]
struct Home;
#[derive(Debug)]
struct Blog;
#[derive(Debug)]
struct Markets;
#[derive(Debug)]
struct About;
//////////////////////
// Components end //
//////////////////////

//////////////////////////
// Custom Bundles start
//////////////////////////

// declare custom entity template (bundle) to hold components (aka unique types)
#[derive(Bundle)]
struct PlayerBundle {
    query_marker: Player,

    health: Health,
    attack_points: AttackPoints,
    old_current: AnimStateTuple, // two elements vector.

    #[bundle]
    sprite_sheet: SpriteSheetBundle
}

#[derive(Bundle)]
struct CreatureBundle {
    query_marker: Creature,

    health: Health,
    attack_points: AttackPoints,

    sprite: SpriteBundle,
}

#[derive(Bundle)]
struct BossBundle {
    query_marker: Boss,

    health: Health,
    attack_points: AttackPoints,

    #[bundle]
    sprite: SpriteBundle,
}

#[derive(Bundle)]
struct StructureBundle {
    query_marker: Structure,

    is_destructible: IsDestructible,
    is_static: IsStatic,

    #[bundle]
    sprite: SpriteBundle  
}

#[derive(Bundle)]
struct Camera2dBundle {
    query_marker: Camera2d,
    #[bundle]
    ortho_bundle: OrthographicCameraBundle
}

#[derive(Bundle)]
struct InGameUiBundle {
    // marks unique bundle for queries (empty struct)
    query_marker: InGameUi,
    // ...
    #[bundle]
    top_bar: NodeBundle,
    #[bundle]
    score: TextBundle,
    #[bundle]
    health: TextBundle, // change into SpriteBundle to make health bar.
    #[bundle]
    wave: TextBundle,
    #[bundle]
    enemies_left: TextBundle,
    #[bundle]
    timer: TextBundle
}

#[derive(Bundle)]
struct MainMenuUiBundle {
    // marks unique bundle for queries (empty struct)
    query_marker: MainMenuUi,
    // ...
    #[bundle]
    start_button: ButtonBundle,
    #[bundle]
    exit_button: ButtonBundle,
    #[bundle]
    header_logo: ImageBundle,
}
////////////////////////
// Custom Bundles end
////////////////////////


// Plugins start
// ...
// Plugins end


// Main //
pub fn run() {
    let mut app = App::build();
    app.init_resource::<Timers>()
//        .init_resource::<Points>()
        .init_resource::<ActionDesc>()
        .init_resource::<SpriteHandles>()
        .init_resource::<AtlasHandles>()
        .init_resource::<Vec<MovementDir>>()
        .insert_resource(WindowDescriptor {
            title: "Zhneeshgame!".to_string(),
            width: WIN_WIDTH,
            height: WIN_HEIGHT,
            resizable: false,
            ..Default::default()
        })
        .insert_resource(
            ClearColor( Color::rgba(0.4, 1.0, 0.8, 0.1)) 
        )
        .insert_resource(
                    Gravity::from(Vec3::new(0.0, -1.81, 0.0))
        );

    // plugins start
    app.add_plugins(DefaultPlugins);

    #[cfg(target_arch = "wasm32")] // add webgl2 plugin if target is webassembly
    app.add_plugin(bevy_webgl2::WebGL2Plugin);

    app.add_plugin( PhysicsPlugin::default() );
    app.add_plugin( LogDiagnosticsPlugin::default() );
    app.add_plugin( FrameTimeDiagnosticsPlugin::default() );
    // plugins end
    app.add_state(AppState::Load)
    // system only runs in load state
        .add_system_set(
            SystemSet::on_enter(AppState::Load)
                .with_system(load_resources.system().label("resources") )
                .with_system(load_textures.system().after("resources"))
        )
        .add_system_set(
            SystemSet::on_update(AppState::Load)
//                .label(AppSystems::ResourceInit)
                .with_system(check_textures.system())
        )
        // system only runs in setup state
        .add_system_set(
            SystemSet::on_enter(AppState::Setup)
                .with_system(build_texture_atlases.system().label("build_atlases"))
                .with_system(init_camera.system().label("init_cam").after("build_atlases") )
                .with_system(init_gui.system().label("init_gui").after("init_cam") )
                .with_system(init_world.system().label("init_world").after("init_gui"))
                .with_system(init_objects.system().label("init_objects").after("init_world"))
                .with_system(init_player.system().label("init_player").after("init_objects"))
        )
        // system only runs in ready state
        .add_system_set(
            SystemSet::on_update(AppState::Ready)
                .with_system(player_input.system().label("input") )
                .with_system(player_animation.system().after("input") )
                .with_system(camera_input.system())
        )
        .run();
}

////////////////////////////////////////
// externally linked function (javascript)
////////////////////////////////////////
#[cfg(target_arch = "wasm32")]
pub fn link_established() -> u8 {
    unsafe {
        let ret: u8 = (PAGE_ID as u8);
        return ret;
    }
}

//////////////////
// Systems start
//////////////////

fn load_resources(
        mut commands: Commands,
        asset_server: Res<AssetServer>,
        mut materials: ResMut<Assets<ColorMaterial>>,
        mut windows: ResMut<Windows>,
        mut actions: ResMut<ActionDesc>,
        mut handles: ResMut<SpriteHandles>,
        mut timers: ResMut<Timers>,
        mut texture_atlases: ResMut<Assets<TextureAtlas>>,
        mut atlas_handles: ResMut<AtlasHandles>,
    ){
    // load texture
    let texture_handle = asset_server.load("road01.png");
    // insert ressource object of type Materials (only one possible) into 'Res' container
    commands.insert_resource(Materials {
        player_materials: materials.add(texture_handle.into())
    });
    
    let mut window = windows.get_primary_mut().unwrap();
    window.set_position(IVec2::new(640, 480));

    // insert ressource object holding window size into container of Ressources
    commands.insert_resource(WinSize {
        w: window.width(),
        h: window.height()
    });

    // timer regulating the PAGE_ID global variable changes. 
    timers.gate_timer = Timer::from_seconds(4.0, true);
    // timer regulating the keydown events that move the player.
    timers.movement_timer = Timer::from_seconds(0.0, true);

    // add timer only. block all movements and jumps until timer complete.
    timers.anim_attack_timer = Timer::from_seconds(0.9, false);
    // timer on jump, but anim also dependent on a landing event. if no landing, switch to 'glide anim' after timer ends until landing
    timers.anim_jump_timer = Timer::from_seconds(0.7, false);

    actions.vel_step = Vec3::new(5.0, 5.0, 5.0);
    actions.vel_max = Vec3::new(20.0, 20.0, 20.0);

    actions.acc_step = Vec3::new(5.0, 5.0, 5.0);
    actions.acc_max = Vec3::new(5.0, 5.0, 5.0);

    // load box texture as globally accessible ressource
    commands.insert_resource(BoxTexture {
        h_texture: asset_server.load("textures/rpg/props/generic-rpg-bridge.png").into()
    });

    // load ground tile textures
    // probably more efficient ways to do this, like load 'tiles' folder at once
    // then differentiate
    handles.grass = asset_server.load_folder("textures/rpg/tiles/grass").unwrap();
    handles.path = asset_server.load_folder("textures/rpg/tiles/path").unwrap();
    handles.home = asset_server.load("textures/rpg/props/generic-rpg-mini-lake.png").into();

    handles.background = Some( asset_server.load_untyped("textures/background02.jpg") );
    
    // ?POSSIBLE? : can afford to 'clone weak' because texture handle is stored on the asset server
    // TODO : fix this! error loading handles in init_world
}


// runs before all other graphics systems
fn init_camera(
    mut commands: Commands,
) {
    // ui camera
    commands.spawn_bundle(UiCameraBundle::default());
    // orthographic camera for perspective, clipping, etc
    commands.spawn_bundle(Camera2dBundle {
        query_marker: Camera2d,
        ortho_bundle: OrthographicCameraBundle::new_2d()
    })
    .insert(
        RigidBody::Dynamic
    )
    .insert(
        Velocity::from_linear(Vec3::X * 0.0)
    );
//    commands.spawn_bundle(OrthographicCameraBundle::new_2d());
    println!("Init cameras!");
}


fn init_mainmenu(
    mut commands: Commands,
    asset_server: Res<AssetServer>, 
    win_size: Res<WinSize>,
    mut materials: ResMut<Assets<ColorMaterial>>
){
    // init mainmenu bundle here //
}


fn init_gui(
    mut commands: Commands,
    asset_server: Res<AssetServer>, 
    win_size: Res<WinSize>,
    mut materials: ResMut<Assets<ColorMaterial>>
){
    // init gui bundle here //
    println!("Init gui!");
    // root gui node
    commands
        .spawn_bundle(NodeBundle {
            style: Style {
                size: Size::new(Val::Percent(100.0), Val::Percent(100.0)),
                display: Display::Flex,
                // horizontal positioning
                justify_content: JustifyContent::SpaceBetween,      
                // vertical positioning
                align_items: AlignItems::FlexEnd, // will draw the nested elements (row and columns) from the top
                ..Default::default()
            },
            material: materials.add(Color::NONE.into()),
            ..Default::default()
        })
        .with_children(|parent| {
            // first row (no material prop, since multiple columns inside this row)
            // in game ui at the top of the screen
            parent
                .spawn_bundle(NodeBundle {
                    style: Style {      // x axis (relative size); y axis(relative size)
                        size: Size::new(Val::Percent(100.0), Val::Percent(10.0)),
                        display: Display::Flex,
                        flex_direction: FlexDirection::Row,
                        flex_wrap: FlexWrap::Wrap,
                        ..Default::default()
                    },
                    ..Default::default()
                })
                .with_children(|parent| {
                    // first column
                    parent
                        .spawn_bundle(NodeBundle {
                            style: Style {      // fill 100%
                                size: Size::new(Val::Percent(30.0), Val::Percent(100.0)),                                  
                                display: Display::Flex,
                                flex_direction: FlexDirection::Column,
                                justify_content: JustifyContent::SpaceEvenly,
                                align_items: AlignItems::Center,
                                
                                ..Default::default()
                            },
                            material: materials.add(Color::rgb(0.35, 0.35, 0.35).into()),
                            ..Default::default()
                        })
                        .with_children(|parent| {
                            // text
                            parent.spawn_bundle(TextBundle {
                                style: Style {
                                    margin: Rect::all(Val::Px(5.0)),
                                    ..Default::default()
                                },
                                text: Text::with_section(
                                    "Text Example",
                                    TextStyle {
                                        font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                                        font_size: 20.0,
                                        color: Color::YELLOW,
                                    },
                                    Default::default(),
                                ),
                                ..Default::default()
                            });
                        });
                    // second column
                    parent
                        .spawn_bundle(NodeBundle {
                            style: Style {      // fill 100%
                                size: Size::new(Val::Percent(30.0), Val::Percent(100.0)),                                  
                                display: Display::Flex,
                                flex_direction: FlexDirection::Column,
                                ..Default::default()
                            },
                            material: materials.add(Color::rgb(0.75, 0.25, 0.15).into()),
                            ..Default::default()
                        });
                    // third column
                    parent
                        .spawn_bundle(NodeBundle {
                            style: Style {
                                size: Size::new(Val::Percent(40.0), Val::Percent(100.0)),                                  
                                display: Display::Flex,
                                flex_direction: FlexDirection::Column,
                                ..Default::default()
                            },
                            material: materials.add(Color::rgb(0.75, 0.15, 0.55).into()),
                            ..Default::default()
                        });
                });
                // end of first row //

            // absolute positioning (quad in the middle )
            /*
            parent
                .spawn_bundle(NodeBundle {
                    style: Style {
                        size: Size::new(Val::Px(200.0), Val::Px(200.0)),
                        position_type: PositionType::Absolute,
                        position: Rect {
                            left: Val::Px(210.0),
                            bottom: Val::Px(10.0),
                            ..Default::default()
                        },
                        border: Rect::all(Val::Px(20.0)),
                        ..Default::default()
                    },
                    material: materials.add(Color::rgb(0.4, 0.4, 0.4).into()),
                    ..Default::default()
                })
                .with_children(|parent| {
                    // nested quad, brighter quad
                    parent.spawn_bundle(NodeBundle {
                        style: Style {
                            size: Size::new(Val::Percent(100.0), Val::Percent(100.0)),
                            ..Default::default()
                        },
                        material: materials.add(Color::rgb(0.8, 0.8, 1.0).into()),
                        ..Default::default()
                    });
                });
            */

            // render order test: reddest in the back, whitest in the front (flex center)
            /*
            parent
                .spawn_bundle(NodeBundle {
                    style: Style {
                        size: Size::new(Val::Percent(100.0), Val::Percent(100.0)),
                        position_type: PositionType::Absolute,
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
                        ..Default::default()
                    },
                    material: materials.add(Color::NONE.into()),
                    ..Default::default()
                })
                .with_children(|parent| {
                    parent
                        .spawn_bundle(NodeBundle {
                            style: Style {
                                size: Size::new(Val::Px(100.0), Val::Px(100.0)),
                                ..Default::default()
                            },
                            material: materials.add(Color::rgb(1.0, 0.0, 0.0).into()),
                            ..Default::default()
                        })
                        .with_children(|parent| {
                            parent.spawn_bundle(NodeBundle {
                                style: Style {
                                    size: Size::new(Val::Px(100.0), Val::Px(100.0)),
                                    position_type: PositionType::Absolute,
                                    position: Rect {
                                        // 20 left from left(center), 20 up from bottom(center)
                                        left: Val::Px(20.0),
                                        bottom: Val::Px(20.0),
                                        ..Default::default()
                                    },
                                    ..Default::default()
                                },
                                material: materials.add(Color::rgb(1.0, 0.3, 0.3).into()),
                                ..Default::default()
                            });
                            parent.spawn_bundle(NodeBundle {
                                style: Style {
                                    size: Size::new(Val::Px(200.0), Val::Px(100.0)),
                                    position_type: PositionType::Absolute,
                                    position: Rect {
                                        left: Val::Px(40.0),
                                        bottom: Val::Px(40.0),
                                        ..Default::default()
                                    },
                                    ..Default::default()
                                },
                                material: materials.add(Color::rgb(1.0, 0.5, 0.5).into()),
                                ..Default::default()
                            });
                            parent.spawn_bundle(NodeBundle {
                                style: Style {
                                    size: Size::new(Val::Px(200.0), Val::Px(200.0)),
                                    position_type: PositionType::Absolute,
                                    position: Rect {
                                        left: Val::Px(60.0),
                                        bottom: Val::Px(60.0),
                                        ..Default::default()
                                    },
                                    ..Default::default()
                                },
                                material: materials.add(Color::rgb(1.0, 0.7, 0.7).into()),
                                ..Default::default()
                            });
                            // alpha test
                            parent.spawn_bundle(NodeBundle {
                                style: Style {
                                    size: Size::new(Val::Px(100.0), Val::Px(100.0)),
                                    position_type: PositionType::Absolute,
                                    position: Rect {
                                        left: Val::Px(80.0),
                                        bottom: Val::Px(80.0),
                                        ..Default::default()
                                    },
                                    ..Default::default()
                                },
                                material: materials.add(Color::rgba(1.0, 0.9, 0.9, 0.4).into()),
                                ..Default::default()
                            });
                        });
                });
            */

            // bevy logo (flex center)
            parent
                .spawn_bundle(NodeBundle {
                    style: Style {
                        size: Size::new(Val::Percent(100.0), Val::Percent(100.0)),
                        position_type: PositionType::Absolute,
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::FlexEnd,
                        ..Default::default()
                    },
                    material: materials.add(Color::NONE.into()),
                    ..Default::default()
                })
                .with_children(|parent| {
                    // bevy logo (image)
                    parent.spawn_bundle(ImageBundle {
                        style: Style {
                            size: Size::new(Val::Px(250.0), Val::Auto),
                            ..Default::default()
                        },
                        material: materials
                            .add(asset_server.load("branding/bevy_logo_dark_big.png").into()),
                        ..Default::default()
                    });
                });
        });
}

/*
    NEEDS to run AFTER player_input. like this we can save old anim_state, and check if changed in player_input here.
    If changed -> set sprite.index to 0 (texture_atlas_handle changed in player_input), and update old animState.
    Else -> just continue incrementing.
*/


fn player_animation(
    time: Res<Time>,
    texture_atlases: Res<Assets<TextureAtlas>>,
    mut atlas_handles: Res<AtlasHandles>,
    mut query: Query<(&mut Timer, Option<&Player>, &mut AnimStateTuple, &mut TextureAtlasSprite, &mut Handle<TextureAtlas>)>,
    mut timers: ResMut<Timers>
)   {
        timers.gate_timer.tick( time.delta() );

        for (mut timer, player, mut tuple, mut sprite, texture_atlas_handle) in query.iter_mut() {
            if let Some(result) = player {
                timer.tick( time.delta() );
                if timer.finished() {
                    if Some(tuple.old) != Some(tuple.current) {
                        sprite.index = 0;
                        tuple.old = tuple.current;
                    }
                    let texture_atlas = texture_atlases.get( (*texture_atlas_handle).clone_weak() ).unwrap();
                    // increment the index of texture atlas with each tick
                    sprite.index = ((sprite.index as usize + 1) % texture_atlas.textures.len()) as u32;
                }
            }
        }
}

fn player_collision(
    time: Res<Time>,
    mut timers: ResMut<Timers>,
    mut events: EventReader<CollisionEvent>,
    mut query: Query<(Entity, Option<&Player>, &mut AnimStateTuple, &mut Velocity)>,
)   {
        for event in events.iter() {
            match event {
                CollisionEvent::Started(d1, d2) => {
                    for (entity, player, mut tuple, mut velocity) in query.iter_mut() {
                        if let Some(player) = player {

                            if d1.rigid_body_entity().id() == entity.id() {
                                // is player.
                                // TODO: check if collision is vertical (downwards)
                                // if yes, check if collision target is rigid body.
                                // then
                                timers.anim_jump_timer.reset();
                            } else if d2.rigid_body_entity().id() == entity.id() {
                                // same as above ...
                                timers.anim_jump_timer.reset();
                            } else {
                                // noone of the colliding entities are the player.
                                // skip ... (for now)
                            }
                        }
                    }
                }
                CollisionEvent::Stopped(d1, d2) => {
                
                }
            }
        }
}


fn load_textures(
    mut sprite_handles: ResMut<SpriteHandles>,
    mut textures: ResMut<Assets<Texture>>,
    asset_server: Res<AssetServer>
)   {
        sprite_handles.player_new.insert( AnimState::Idle, asset_server.load_folder("ninja/png/Idle").unwrap());
        sprite_handles.player_new.insert( AnimState::Run, asset_server.load_folder("ninja/png/Run").unwrap());
        sprite_handles.player_new.insert( AnimState::Jump, asset_server.load_folder("ninja/png/Jump").unwrap());
        sprite_handles.player_new.insert( AnimState::Attack, asset_server.load_folder("ninja/png/Attack").unwrap());
        sprite_handles.player_new.insert( AnimState::Glide, asset_server.load_folder("ninja/png/Glide").unwrap());

        
}


fn check_textures(
    mut state: ResMut<State<AppState>>,
    sprite_handles: ResMut<SpriteHandles>,
    asset_server: Res<AssetServer>,
    texture_atlases: Res<Assets<TextureAtlas>>,
    atlas_handles: Res<AtlasHandles>
)   {
        let mut finished_player : bool = false;
        let mut finished_tiles : bool = false;
        let mut finished_home : bool = false;
        let mut finished_background : bool = false;

        for (anim_type, vec) in sprite_handles.player_new.iter() {
            if asset_server.get_group_load_state(
                vec.iter().map(|handle| handle.id)
            ) != LoadState::Loaded {
                // loading not finished yet for this animation type
                finished_player = false;
                break;
            } else {
                finished_player = true;
            }
        }

        if finished_player {
            println!("Loading of player textures is finished!");
        }

        if let LoadState::Loaded =
            asset_server.get_group_load_state(sprite_handles.grass.iter().map(|handle| handle.id))
        {
            finished_tiles = true;
            println!("Loading of tile textures is finished!");
        }

        if let LoadState::Loaded =
            asset_server.get_load_state(sprite_handles.home.id )
        {
            finished_home = true;
            println!("Loading of home texture is finished!");
        }

        if let LoadState::Loaded =
            asset_server.get_load_state(sprite_handles.background.as_ref().unwrap().id)
        {
            finished_background = true;
            println!("Loading of background texture is finished!");
        }

        if ( finished_player && finished_tiles && finished_home && finished_background ) {
            println!("Loading of all textures is finished!");
            state.set(AppState::Setup).unwrap();
        }
}


fn build_texture_atlases(
    mut commands: Commands,
    sprite_handles: Res<SpriteHandles>,
    mut atlas_handles: ResMut<AtlasHandles>,
    mut texture_atlases: ResMut<Assets<TextureAtlas>>,
    mut textures: ResMut<Assets<Texture>>,
    asset_server: Res<AssetServer>,
)   {
        let mut texture_atlas_builder : TextureAtlasBuilder;

        for (anim_type, vec) in sprite_handles.player_new.iter() {

            texture_atlas_builder = TextureAtlasBuilder::default();
            for texture_handle in vec.iter() {
                let texture = textures.get( texture_handle.clone_weak().typed::<Texture>() ).unwrap();
                texture_atlas_builder.add_texture( texture_handle.clone_weak().typed::<Texture>(), texture );
            }
            let texture_atlas : TextureAtlas = texture_atlas_builder.finish(&mut textures).unwrap();
            // add texture atlas to texture atlases resource vector
            let atlas_handle : Handle<TextureAtlas> = texture_atlases.add(texture_atlas);
            atlas_handles.player.insert(*anim_type, atlas_handle);
        }
}


fn init_world(
    mut commands: Commands,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut textures: ResMut<Assets<Texture>>,
    mut texture_atlases: ResMut<Assets<TextureAtlas>>,
    sprite_handles: Res<SpriteHandles>,
    win_size: Res<WinSize>,
)   {
        // I. create the ground tiles
        // from bottom left to top right ( draw whole screen )
//        let mut incr = 0;
        let mut x_counter : u32 = 0;
        let mut y_counter : u32 = 0;                        // half extend
        let mut x_start: f32 = -1.0 * (WIN_WIDTH / 2.0) + MOVE_DISTANCE_X;
        let mut y_start: f32 = -1.0 * (WIN_HEIGHT / 2.0) + MOVE_DISTANCE_Y;
        //                          desired size    actual size
        let scalefactor_x : f32 = TILE_GOALSIZE / TILE_ACTUALSIZE;
        let scalefactor_y : f32 = TILE_GOALSIZE / TILE_ACTUALSIZE;
/* 
        let mut texture_atlas_builder = TextureAtlasBuilder::default();
        // handle does not seem to exist ?
        for handle in sprite_handles.grass.iter() {
            println!("Found one texture! in grass handles!");
            let texture = textures.get( handle.clone_weak().typed::<Texture>() ).unwrap();
            texture_atlas_builder.add_texture( handle.clone_weak().typed::<Texture>(), texture );
        }
*/ 
        let mut incr: usize = 0;

        /* 
        // Infinite loop
        loop {
            if(y_counter >= TILES_Y) {
                x_counter += 1;
                y_counter = 0;
                if (x_counter >= TILES_X) {
                    // end while loop
                    break;
                }
            }
            commands
            .spawn_bundle( SpriteBundle {
                material: materials.add( ColorMaterial{
                    texture: Some( sprite_handles.grass[incr].clone_weak().typed::<Texture>() ),
                    ..Default::default()
                }),
                transform: Transform {
                    // TODO: modify translation every loop
                    translation: Vec3::new(
                                            (x_counter as f32)* TILE_GOALSIZE + x_start, // x coord
                                            (y_counter as f32)* TILE_GOALSIZE + y_start,  // y coord
                                            0.0
                                        ),
                    scale: Vec3::new(scalefactor_x, scalefactor_y, 1.0),
                    ..Default::default()
                },
                ..Default::default()
            });
            

            // pick random index out of the vector of texture handles
            incr = ( ( rand::random::<usize>()) % sprite_handles.grass.len() ) as usize;
            y_counter += 1;
        }
        */

        commands
            .spawn_bundle( SpriteBundle {
                material: materials.add( ColorMaterial{
                    texture: Some( sprite_handles.background.as_ref().unwrap().clone_weak().typed::<Texture>() ),
                    ..Default::default()
                }),
                transform: Transform {
                    // TODO: modify translation every loop
                    translation: Vec3::new(
                                          0.0,
                                          0.0,
                                          0.0
                                        ),
                    scale: Vec3::new(1.0, 1.0, 1.0),
                    ..Default::default()
                },
                ..Default::default()
            });
        
        println!("Finished spawning tiles!");
}


fn init_player(
    mut state: ResMut<State<AppState>>,
    mut commands: Commands,
    sprite_handles: Res<SpriteHandles>,
    atlas_handles: Res<AtlasHandles>,
    texture_atlases: Res<Assets<TextureAtlas>>,
    textures: ResMut<Assets<Texture>>,
//    asset_server: Res<AssetServer>,
    win_size: Res<WinSize>,
)   {
        println!("Init player!");
        // I. build texture atlas (sprite sheet) from textures

        let text_handle : Handle<Texture> = sprite_handles.player_new.get(&AnimState::Idle).unwrap()[0].clone_weak().typed::<Texture>();
        // get first sprite texture
        let first_sprite : &Texture = textures.get(text_handle).unwrap();

        // scale parameters that bring us 24px width and 24px height

        // TODO: need to make this constant (ratio of TILE_GOALSIZE to win size.)
        // desired height and width of player. a multiple of 24px each.
        let desired_with: f32 = TILE_GOALSIZE * 2.0;
        let desired_height: f32 = TILE_GOALSIZE * 4.0;

        // effective scale of player, relation between desired size (as multiple of 24px tile size) 
        // and actual texture size.
        let effective_scale_x : f32 = desired_with / (first_sprite.size.width as f32);
        let effective_scale_y : f32 = desired_height / (first_sprite.size.height as f32);

        // set up a scene to display our texture atlas //
                                                    // half extend to right
        let mut x_start: f32 = -1.0 * (WIN_WIDTH / 2.0) + MOVE_DISTANCE_X;
        let mut y_start: f32 = -1.0 * (WIN_HEIGHT / 2.0) + MOVE_DISTANCE_Y;

        // TI
        let mut half_tile_offset_x: f32 = TILE_GOALSIZE / 2.0;
        let mut half_tile_offset_y: f32 = TILE_GOALSIZE / 2.0;

        // II. spawn player sprite sheet bundle
        commands
        .spawn_bundle(PlayerBundle {
            query_marker: Player,
            health: Health(100.0),
            attack_points: AttackPoints(1.0),

            old_current: AnimStateTuple {
                            old: Some(AnimState::Idle), 
                            current: Some(AnimState::Idle)
            },
            sprite_sheet: SpriteSheetBundle {
                texture_atlas: atlas_handles.player.get(&AnimState::Idle).unwrap().clone(),
                transform: Transform {
                    translation: Vec3::new( -TILE_UNIT_TRANSLATION*24.0, 0.0, 1.0 ),
                    scale: Vec3::new(effective_scale_x, effective_scale_y, 1.0),
                    ..Default::default()
                },
                visible: Visible {
                    is_visible: true,
                    is_transparent: true
                },
                ..Default::default()
            },
        })
        .insert( CollisionShape::Cuboid {
            half_extends: Vec3::new((desired_with / 2.0), (desired_height / 2.0), 1.0),
            border_radius : Some(0.0) // optional argument (type Option)
        })
        .insert( RigidBodyEnum::Dynamic )
        .insert(RotationConstraints::lock()) // disallow rotation around axis
        .insert( PhysicMaterial {
            friction: 0.0, 
            density: 1.0,
            restitution: 0.0,
            ..Default::default()
        })
        // insert timer as member of PlayerBundle instance
        .insert(Timer::from_seconds(0.15, true))
        .insert(TexSize {
            w: (first_sprite.size.width as f32),
            h: (first_sprite.size.height as f32),
            scale_w: effective_scale_x,
            scale_h: effective_scale_y
        })
        .insert(
            Velocity::from_linear(Vec3::X * 0.0)
        )
        .insert(
            Acceleration::from_linear(Vec3::X * 0.0)
        );
        ///////////////////////////////
        // set program state to 'Ready'
         //////////////////////////////
        state.set(AppState::Ready).unwrap();
}


fn init_opponent(
    mut commands: Commands,
    asset_server: Res<AssetServer>, 
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut textures: ResMut<Assets<Texture>>,
    handles: Res<SpriteHandles>,
)   {

}


fn init_objects(
    mut commands: Commands,
    asset_server: Res<AssetServer>, 
    win_size: Res<WinSize>,
    box_texture: Res<BoxTexture>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut textures: ResMut<Assets<Texture>>,
    handles: Res<SpriteHandles>,
)   {

        // ?TODO? : load all required textures in load_ressources, then use AssetEvent here
        // to make sure asset is loaded before accessing Handle<Texture/ColorMaterial>
        // because AssetServer loads assets ---asynchronously---
        // https://bevy-cheatbook.github.io/features/assets.html

        let mut loading = true;
        while(loading) {
            if ( textures.contains( box_texture.h_texture.clone() )) {
                println!("Assets contain this texture after load!");
                loading = false;
            } else {
                println!("Still loading!");
            }
        }
        let texture : &Texture = textures.get(box_texture.h_texture.clone()).unwrap();

        let width : f32 = texture.size.width as f32;
        let height : f32 = texture.size.height as f32;

        // TODO: insert into vector ressource holding top left and bottom right corners here....

        // desired height and width of player. a multiple of 24px each.
        let desired_with: f32 = TILE_GOALSIZE * 4.0;
        let desired_height: f32 = TILE_GOALSIZE * 4.0;
       
        // effective scale of player, relation between desired size (as multiple of 24px tile size) 
        // and actual texture size.
        let effective_scale_x : f32 = desired_with / width;
        let effective_scale_y : f32 = desired_height / height;

        let mut point_tl : Vec3 = Vec3::new(0.0, 0.0, 0.0);
        let mut point_br : Vec3 = Vec3::new(0.0, 0.0, 0.0);

        let translation_x :  f32 =  -TILE_UNIT_TRANSLATION*5.0;
        let translation_y : f32 = -TILE_UNIT_TRANSLATION*5.0;
        let translation : Vec3 = Vec3::new(translation_x, translation_y, 0.0);

        let color_material = ColorMaterial {
            texture: Some(box_texture.h_texture.clone()),
            ..Default::default()
        };

        // bridge
        commands
            .spawn_bundle( SpriteBundle {
                material: materials.add( color_material ),
                transform: Transform {
                    translation: Vec3::new(
                                                translation_x,
                                                translation_y, 
                                                1.0
                                            ),
                    scale: Vec3::new(effective_scale_x, effective_scale_y, 1.0),
                    ..Default::default()
                },
                ..Default::default()
            })
            
            .insert( CollisionShape::Cuboid {
                half_extends: Vec3::new( desired_with / 2.0, desired_height / 2.0, 1.0),
                border_radius : None // optional argument (type Option)
            })
            .insert( RigidBodyEnum::Static )
            .insert( PhysicMaterial {
                friction: 0.0,
                density: 0.0,
                restitution: 0.0,
                ..Default::default()
            })
            
            .insert( TexSize {
                w: width,
                h: height,
                scale_w: effective_scale_x,
                scale_h: effective_scale_y
            });

        point_tl = translation - (Vec3::new( ((width * effective_scale_x) / 2.0), 0.0, 0.0));
        point_tl = point_tl + (Vec3::new( 0.0, ((height * effective_scale_y) / 2.0), 0.0));
        point_br = translation + (Vec3::new( ((width * effective_scale_x) / 2.0), 0.0, 0.0));
        point_br = point_br - (Vec3::new( 0.0, ((height * effective_scale_y) / 2.0), 0.0));

//        [cfg(target_arch = "wasm32")]
//        points.hash_map.insert( GateIdentifier::Blog, (point_tl, point_br) );

        // pond/lake (no collisions for now)
        commands
            .spawn_bundle( SpriteBundle {
                material: materials.add( ColorMaterial {
                    texture: Some( handles.home.clone() ),
                    ..Default::default()
                }),
                transform: Transform {
                    translation: Vec3::new(TILE_UNIT_TRANSLATION*10.0, TILE_UNIT_TRANSLATION*20.0, 1.0), //////// <------
                    scale: Vec3::new(2.0, 2.0, 0.0),
                    ..Default::default()
                },
                visible: Visible {
                    is_visible: true,
                    is_transparent: false
                },
                ..Default::default()
            });

        /*

            TODO: platforms, obstacles, dynamic objects, etc. here.

        */
/*
            .insert( CollisionShape::Cuboid {
                half_extends: Vec3::new( (width / 2.0), (height / 2.0), 1.0),
                border_radius : Some(0.0) // optional argument (type Option)
            })
            .insert( RigidBody::Dynamic );
*/
}


fn player_input(
    time: Res<Time>,
    mut actions: ResMut<ActionDesc>,
    mut atlas_handles: Res<AtlasHandles>,
    keys: Res<Input<KeyCode>>,
    mut query: Query<( Option<&Player>, &mut AnimStateTuple, &mut Transform, Option<&mut Velocity>, &mut TextureAtlasSprite, &mut Handle<TextureAtlas> )>,
//    points: Res<Points>,
    mut timers: ResMut<Timers>
)   {
        let mut moving: bool = false;
        let mut aborted: bool = false;

        for ( player, mut tuple, mut transform, mut velocity, mut sprite, mut handle_atlas ) in query.iter_mut() {
            let mut vel = velocity.unwrap();
                // if timers not started or not finished yet.
            if (timers.anim_jump_timer.elapsed_secs() > 0.0) && !timers.anim_jump_timer.finished() {
                timers.anim_jump_timer.tick( time.delta() );
                // TODO; check for attack key just pressed. to initiate jump_attack animation here.
                // block all movement
                aborted = true;
                break;
            } else if timers.anim_jump_timer.finished() {
                // switch to 'glide anim'
                tuple.current = Some(AnimState::Glide);
                *handle_atlas = atlas_handles.player.get( &tuple.current.unwrap() ).unwrap().clone();
//                vel.linear = Vec3::new(0.0, 0.0, 0.0);
                aborted = true;
                break;
                // TODO: reset jump_timer on vertical collision in a collision handling system.
            } else {
            }

            if (timers.anim_attack_timer.elapsed_secs() > 0.0) && !timers.anim_attack_timer.finished() {
                timers.anim_attack_timer.tick( time.delta() );
                aborted = true;
                break;
            } else if timers.anim_attack_timer.finished() {
                // reset timer, and continue as usual.
                timers.anim_attack_timer.reset();
            } else {

            }
            
            if let Some(result) = player {
                if keys.just_pressed(KeyCode::A) {
                    println!("just pressed A!");
                }
                if keys.just_pressed(KeyCode::D) {
                    println!("just pressed D!");
                }
                if keys.just_pressed(KeyCode::W) {
                    println!("just pressed W!");
                }
                if keys.just_pressed(KeyCode::S) {
                    println!("just pressed S!");
                }

                if (
                    keys.pressed(KeyCode::A) || keys.pressed(KeyCode::D) || keys.pressed(KeyCode::W) || keys.pressed(KeyCode::S) ) {
                        tuple.current = Some(AnimState::Run);
                        moving = true;
                } else if (
                        ( keys.just_released(KeyCode::A) || keys.just_released(KeyCode::D) || keys.just_released(KeyCode::W) || keys.just_released(KeyCode::S) )
                    &&
                        ( !keys.pressed(KeyCode::Space) || !keys.pressed(KeyCode::LShift) )
                ) {
                        tuple.current = Some(AnimState::Idle);
                        moving = false;
                } else {
                }

                // keys just pressed take precedence in animation state over those recently pressed.
                if keys.just_pressed(KeyCode::LShift) {
                    tuple.current = Some(AnimState::Jump);
                    timers.anim_jump_timer.tick( time.delta() );
                    *handle_atlas = atlas_handles.player.get( &tuple.current.unwrap() ).unwrap().clone();
                    vel.linear += Vec3::new(0.0, 50.0, 0.0);
                    aborted = true;
                    break;
                } else if keys.just_released(KeyCode::LShift) {
                }

                if keys.just_pressed(KeyCode::Space) {
                    tuple.current = Some(AnimState::Attack);
                    timers.anim_attack_timer.tick( time.delta() );
                    *handle_atlas = atlas_handles.player.get( &tuple.current.unwrap() ).unwrap().clone();
                    vel.linear = Vec3::new(0.0, 0.0, 0.0);
                    aborted = true;
                    break;
                } else if keys.just_released(KeyCode::Space) {
                }

                // just moving, or idle.
                *handle_atlas = atlas_handles.player.get( &tuple.current.unwrap() ).unwrap().clone();
/* 
                if (*anim_state == AnimState::Idle || ((*anim_state == AnimState::Attack) && !moving) ) {
                    break;
                }
*/
                if keys.pressed(KeyCode::A) && keys.just_pressed(KeyCode::A) {
                    // just pressed. has been held for 1 or 2 frames only.
                    if !( vel.linear * Vec3::X <= Vec3::new(-100.0, 0.0, 0.0) ) {
                        vel.linear -= Vec3::new(120.0, 0.0, 0.0);
                    }
//                    actions.move_dir = Some(MovementDir::MoveLeft);
                    if !sprite.flip_x {
                        sprite.flip_x = true;
                    }
                    println!("just pressed and held down!");

                } else if keys.pressed(KeyCode::A) && !keys.just_pressed(KeyCode::A) {
                    // has been pressed for more than 1 (or 2 ) frames, increase 
                    if !( vel.linear * Vec3::X <= Vec3::new(-100.0, 0.0, 0.0) ) {
                        vel.linear -= Vec3::new(120.0, 0.0, 0.0);
                    }

                    println!("held down, but not just pressed!");
                } else if keys.just_released(KeyCode::A) {
                    // just released
                    if !( vel.linear * Vec3::X >= Vec3::new(0.0, 0.0, 0.0) ) {
                        // decrease lateral velocity to 0.0
                        vel.linear = vel.linear + (vel.linear * Vec3::X).abs();
                    }
                    println!("just released!");
                } else {
                    // ...
                }

                if keys.pressed(KeyCode::D) && keys.just_pressed(KeyCode::D) {
                    // just pressed. has been held for 1 or 2 frames only.
                    if !( vel.linear * Vec3::X >= Vec3::new(100.0, 0.0, 0.0) ) {
                        vel.linear += Vec3::new(120.0, 0.0, 0.0);
                    }

                    if sprite.flip_x {
                        sprite.flip_x = false;
                    }
                } else if keys.pressed(KeyCode::D) && !keys.just_pressed(KeyCode::D) {
                    // has been pressed for more than 1 (or 2 ) frames, increase 
                    if !( vel.linear * Vec3::X >= Vec3::new(100.0, 0.0, 0.0) ) {
                        vel.linear += Vec3::new(120.0, 0.0, 0.0);
                    }
                } else if keys.just_released(KeyCode::D) {
                    // just released
                    if !( vel.linear * Vec3::X <= Vec3::new(0.0, 0.0, 0.0) ) {
                        // decrease lateral velocity to 0.0
                        vel.linear = vel.linear - (vel.linear * Vec3::X).abs();
                    }
                } else {
                    // ...
                }

                if keys.pressed(KeyCode::W) && keys.just_pressed(KeyCode::W) {
                    // just pressed. has been held for 1 or 2 frames only.
                    if !( vel.linear * Vec3::Y >= Vec3::new(0.0, 100.0, 0.0) ) {
                        vel.linear += Vec3::new(0.0, 100.0, 0.0);
                    }
                } else if keys.pressed(KeyCode::W) && !keys.just_pressed(KeyCode::W) {
                    // has been pressed for more than 1 (or 2 ) frames, increase 
                    if !( vel.linear * Vec3::Y >= Vec3::new(0.0, 100.0, 0.0) ) {
                        vel.linear += Vec3::new(0.0, 100.0, 0.0);
                    }
                } else if keys.just_released(KeyCode::W) {
                    // just released
                    if !( vel.linear * Vec3::Y <= Vec3::new(0.0, 0.0, 0.0) ) {
                        // decrease lateral velocity to 0.0
                        vel.linear = vel.linear - (vel.linear * Vec3::Y).abs();
                    }
                } else {
                }

                if keys.pressed(KeyCode::S) && keys.just_pressed(KeyCode::S) {
                    // just pressed. has been held for 1 or 2 frames only.
                    if !( vel.linear * Vec3::Y <= Vec3::new(0.0, -100.0, 0.0) ) {
                        vel.linear -= Vec3::new(0.0, 100.0, 0.0);
                    }
                } else if keys.pressed(KeyCode::S) && !keys.just_pressed(KeyCode::S) {
                    // has been pressed for more than 1 (or 2 ) frames, increase 
                    if !( vel.linear * Vec3::Y <= Vec3::new(0.0, -100.0, 0.0) ) {
                        vel.linear -= Vec3::new(0.0, 100.0, 0.0);
                    }
                } else if keys.just_released(KeyCode::S) {
                    // just released
                    if !( vel.linear * Vec3::Y >= Vec3::new(0.0, 0.0, 0.0) ) {
                        // decrease lateral velocity to 0.0
                        vel.linear = vel.linear + (vel.linear * Vec3::Y).abs();
                    }
                } else {
                }
            }
        }

/* 
        if (timers.gate_timer.elapsed_secs() % 4.0) >= 2.0 {
            unsafe {
                if (PAGE_ID != GateIdentifier::None) {
                    PAGE_ID = GateIdentifier::None;
                    println!("Timer finished. setting Gateidentifier to none!");
                    println!("{}", format!("elasped secs: {}", timers.gate_timer.elapsed_secs()));
                    //timers.gate_timer.tick( Duration::from_secs(2) ); // or reset timer?
                    timers.gate_timer.reset();
                }
            }
        }
*/
        // (v.is_empty == true; but no effect on reserved capacity) 
}

fn camera_input(
    mut query: Query<(Option<&Player>, Option<&Camera2d>, &mut Transform, Option<&mut OrthographicProjection>, &mut Velocity)>,
)   {
        // take reference of camera and player pos from the query.
        let mut camera_transform: Option<Mut<Transform>> = None;
        let mut camera_proj: Option<Mut<OrthographicProjection>> = None;
        let mut player_transform : Option<Mut<Transform>> = None;
        let mut camera_velocity: Option<Mut<Velocity>> = None;

        // TODO: if camera reaches a certain point (left<->right), move camera.
        for (opt_player, camera, mut transform, mut proj, mut velocity) in query.iter_mut() {
            if let Some(player) = opt_player {
                player_transform = Some(transform);
            } else if let Some(cam) = camera {
                camera_transform = Some(transform);
                camera_proj = proj; 
                camera_velocity = Some(velocity);
            }
        }

        // closures are awesome.
        let mut cam_transform = camera_transform.unwrap();
        let cam_proj= camera_proj.unwrap();
        let p_transform = player_transform.unwrap();
        let mut cam_velocity = camera_velocity.unwrap();

        if (p_transform.translation * Vec3::X) > 
                (cam_transform.translation * Vec3::X + Vec3::new(cam_proj.right - 50.0, 0.0, 0.0).abs()) 
        {
            // apply velocity to camera until it is centered.
                cam_velocity.linear = Vec3::new(100.0, 0.0, 0.0);

        } else if p_transform.translation * Vec3::X <
                    (cam_transform.translation * Vec3::X - Vec3::new(cam_proj.left + 50.0, 0.0, 0.0).abs()) {

            // apply velocity to camera until it is centered.
            cam_velocity.linear = Vec3::new(-100.0, 0.0, 0.0);
        }

        // camera is adjusting
        if cam_velocity.linear * Vec3::X > Vec3::new(0.0, 0.0, 0.0) {
            // camera has reached horizontal center. stop.
            if cam_transform.translation * Vec3::X >= p_transform.translation * Vec3::X {
                cam_velocity.linear = Vec3::new(0.0, 0.0, 0.0);
            }
        } else if cam_velocity.linear * Vec3::X < Vec3::new(0.0, 0.0, 0.0) {
            // cameras has reached horizontal center. stop.
            if cam_transform.translation * Vec3::X <= p_transform.translation * Vec3::X {
                cam_velocity.linear = Vec3::new(0.0, 0.0, 0.0);
            }
        }


}

// both player and hostile projectiles.
fn projectile_manager(
    time: Res<Time>,
    mut actions: ResMut<ActionDesc>,
    keys: Res<Input<KeyCode>>,
    mut query: Query<(Option<&Player>, &mut TextureAtlasSprite, &Handle<TextureAtlas>)>,
    texture_atlases: Res<Assets<TextureAtlas>>,
    mut timers: ResMut<Timers>
)   {
        for (player, mut sprite, texture_atlas_handle) in query.iter_mut() {
            // TODO: condition: if animation = throw, continue, else break.
            // -> activate throw only on just_pressed.
            let texture_atlas = texture_atlases.get(texture_atlas_handle).unwrap();
            // increment the index of texture atlas with each tick
            sprite.index = ((sprite.index as usize + 1) % texture_atlas.textures.len()) as u32;
        }
        // TODO: switch to other texture atlas containing attack sprites.
        // HOW: query texture_atlas field of SpriteSheetBundle and assign to it.
        // TODO: also spawn moving weapon sprite (throw attack).
}

// Systems end