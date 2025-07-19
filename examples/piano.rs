use std::{error, fs::File, io::Write, path::Path, sync::Arc};

use bevy::{
    ecs::relationship::RelationshipSourceCollection, math::primitives::Cuboid, pbr::AmbientLight, prelude::*, render::{prelude::Msaa, view::NoFrustumCulling}
};

use bevy_eventlistener::prelude::*;
use bevy_picking::prelude::*;


use bevy_egui::{egui::{self, Color32, FontData, FontDefinitions, FontFamily}, EguiContexts, EguiPlugin, EguiPrimaryContextPass};

use midi_piano_visualizer::prelude::*;

use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
struct TransformSaveData {
    translation: [f32; 3],
    rotation: [f32; 4],
    scale: [f32; 3],
}
#[derive(Serialize, Deserialize)]
struct SaveData {
    transforms: TransformSaveData
}
use std::fs;

const CAMERA_POS: Vec3 = Vec3::new(0., 5., -16.);

#[derive(Resource)]
struct SelectedColor(Color32);


fn main() {
    
    App::new()
     .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                transparent: true, // ← 透明を有効化
                decorations: true, // ← 枠
                #[cfg(target_os = "macos")]
                composite_alpha_mode: CompositeAlphaMode::PostMultiplied,
                #[cfg(target_os = "linux")]
                composite_alpha_mode: CompositeAlphaMode::PreMultiplied,
                ..default()
            }),
            ..default()
        }))
     .add_plugins((
            DefaultPickingPlugins,         // 入力とイベント処理
            //InteractionPlugin,     // イベントのバブリング
            MeshPickingPlugin,     // ← これが RayMap を提供する！
        ))
        
        .add_plugins(EguiPlugin::default())
        .insert_resource(AmbientLight {
            color: Color::WHITE,
            brightness: 1.0 / 5.0f32,
            ..default()
        })
        .add_plugins(MidiInputPlugin)
        .init_resource::<MidiInputSettings>()
        .add_plugins(MidiOutputPlugin)
        .init_resource::<MidiOutputSettings>()
        .insert_resource(SelectedColor(Color32::WHITE))
        .insert_resource(ClearColor(Color::NONE))
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                handle_midi_input,
                connect_to_first_input_port,
                connect_to_first_output_port,
                display_press,
                update_pressed_note,
                display_release,
                key_input_system,
            )).add_systems(EguiPrimaryContextPass, (
                ui_menu_bar
            ))
            .insert_resource(WindowState { show_tool_window: true })

        .run();
}

#[derive(Component, Debug)]
#[component(storage = "Table")] 
struct Key {
    key_val: String,
    key_num: u8,
    oct: u8,
    is_note_created: bool,
    y_reset: f32,
}

#[derive(Component)]
struct PressedKey;

#[derive(Component)]
#[component(storage = "Table")] 
struct Note{
    key_num: u8,
    oct: u8,
    length: f32,
    delta_x: f32,
    is_creating: bool
}
#[derive(Resource)]
struct WindowState {
    show_tool_window: bool,
}


fn ui_menu_bar(mut contexts: EguiContexts, mut state: ResMut<WindowState>, mut selected: ResMut<SelectedColor>) {
    let mut show = state.show_tool_window;
    egui::Window::new("Tool").open(&mut show).show(contexts.ctx_mut().unwrap(), |ui| {
        // カラーピッカーを表示
        ui.label("Choose Notes Color");
        egui::color_picker::color_edit_button_srgba(ui, &mut selected.0, egui::color_picker::Alpha::OnlyBlend);

        ui.separator();
        ui.colored_label(selected.0, "selected");

        ui.label("Menu");
        if ui.button("Quit Menu (m)").clicked() {
            state.show_tool_window = false; 
        }
    });
}

fn setup_ui(mut cmds: Commands, mut egui_ctx: EguiContexts,) {
    let mut fonts = FontDefinitions::default();

    // フォントデータを登録（Arcで包む）
    fonts.font_data.insert(
        "my_font".to_owned(),
        Arc::new(FontData::from_static(include_bytes!("../assets/fonts/FiraSans-Bold.ttf"))),
    );


    // フォントファミリーに割り当て
    fonts.families.entry(FontFamily::Proportional).or_default().insert(0, "my_font".to_owned());
    fonts.families.entry(FontFamily::Monospace).or_default().push("my_font".to_owned());

    egui_ctx.ctx_mut().unwrap().set_fonts(fonts);
}

#[rustfmt::skip]
fn setup(
    mut cmds: Commands,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,

) {
    let mid = -6.3;

    // light
    cmds.spawn((PointLight {
            shadows_enabled: true,
            ..default()
        }
, Transform::from_xyz(0.0, 6.0, 0.0)));

// light
    cmds.spawn((PointLight {
            shadows_enabled: true,
            ..default()
        }
, Transform::from_xyz(0.0, 6.0, mid)));

// light
    cmds.spawn((PointLight {
            shadows_enabled: true,
            ..default()
        }
, Transform::from_xyz(0.0, 6.0, mid*2.0)));

    //Camera
    cmds.spawn((
        Camera3d::default(), 
        Msaa::Sample4,
        Transform::from_xyz(CAMERA_POS.x, CAMERA_POS.y, CAMERA_POS.z).looking_at(Vec3::new(0., 0., mid), Vec3::Y)
    ));

    let pos: Vec3 = Vec3::new(0., 0., 0.);

    let mut black_key: Handle<Mesh> = asset_server.load("models/black_key.gltf#Mesh0/Primitive0");
    let mut white_key_0: Handle<Mesh> = asset_server.load("models/white_key_0.gltf#Mesh0/Primitive0");
    let mut white_key_1: Handle<Mesh> = asset_server.load("models/white_key_1.gltf#Mesh0/Primitive0");
    let mut white_key_2: Handle<Mesh> = asset_server.load("models/white_key_2.gltf#Mesh0/Primitive0");
    let b_mat = materials.add(Color::srgb(0.1, 0.1, 0.1));
    let w_mat = materials.add(Color::srgb(1.0, 1.0, 1.0));

    //Create keyboard layout
    let pos_black = pos + Vec3::new(0., 0.06, 0.);
    
    for i in 0..8 {
        spawn_note(&mut cmds, &w_mat, 0.00, pos, &mut white_key_0, i, "C", 0);
        spawn_note(&mut cmds, &b_mat, 0.15, pos_black, &mut black_key, i, "C#/Db", 1);
        spawn_note(&mut cmds, &w_mat, 0.27, pos, &mut white_key_1, i, "D", 2);
        spawn_note(&mut cmds, &b_mat, 0.39, pos_black, &mut black_key, i, "D#/Eb", 3);
        spawn_note(&mut cmds, &w_mat, 0.54, pos, &mut white_key_2, i, "E", 4);
        spawn_note(&mut cmds, &w_mat, 0.69, pos, &mut white_key_0, i, "F", 5);
        spawn_note(&mut cmds, &b_mat, 0.85, pos_black, &mut black_key, i, "F#/Gb", 6);
        spawn_note(&mut cmds, &w_mat, 0.96, pos, &mut white_key_1, i, "G", 7);
        spawn_note(&mut cmds, &b_mat, 1.08, pos_black, &mut black_key, i, "G#/Ab", 8);
        spawn_note(&mut cmds, &w_mat, 1.19, pos, &mut white_key_1, i, "A", 9);
        spawn_note(&mut cmds, &b_mat, 1.31, pos_black, &mut black_key, i, "A#/Bb", 10);
        spawn_note(&mut cmds, &w_mat, 1.46, pos, &mut white_key_2, i, "B", 11);
    }
}

fn on_up(_out:  Trigger<Pointer<Released>>, mut commands: Commands) {
    commands.entity(_out.target()).remove::<PressedKey>();
}

fn on_out(_out:  Trigger<Pointer<Out>>, mut commands: Commands) {
    commands.entity(_out.target()).remove::<PressedKey>();   
}

fn on_down(_down: Trigger<Pointer<Pressed>>, mut commands: Commands) {
    commands.entity(_down.target()).insert(PressedKey);
}

fn spawn_note(
    commands: &mut Commands,
    mat: &Handle<StandardMaterial>,
    offset_z: f32,
    pos: Vec3,
    asset: &mut Handle<Mesh>,
    oct: u8,
    key: &str,
    key_num: u8
) {
    
commands.spawn((
        Mesh3d ( asset.clone()),
        MeshMaterial3d 
            (mat.clone()),
            Transform {
                translation: Vec3::new(pos.x, pos.y, pos.z - offset_z - (1.61 * oct as f32)),
                scale: Vec3::new(10., 10., 10.),
                ..Default::default()
            },
        Key {
            key_val: format!("{}{}", key, oct),
            key_num: key_num,
            oct: oct, 
            is_note_created: false,
            y_reset: pos.y,
        })).observe(on_up).observe(on_out).observe(on_down);

}

fn display_press(mut commands: Commands, mut query: Query<(&mut Transform, &mut Key), With<PressedKey>>, mut meshes: ResMut<Assets<Mesh>>,mut materials: ResMut<Assets<StandardMaterial>>, selected: Res<SelectedColor>) {
    for (mut t, mut k) in &mut query {
        t.translation.y = -0.05;
        if ! k.is_note_created {
            println!("{}{}{}", selected.0.r(), selected.0.g(), selected.0.b());
            k.is_note_created = true;
            // Cuboid（直方体）を生成
            let cuboid = Cuboid::new(0.1, 0.1, 0.1); // 幅・高さ・奥行き
            let material_handle = materials.add(StandardMaterial {
            base_color: Color::srgb(
                selected.0.r() as f32/255.0,
                selected.0.g() as f32/255.0,
                selected.0.b() as f32/255.0,
            ),
            ..Default::default()
        });             

            commands.spawn((
                Mesh3d (meshes.add(cuboid)),
                MeshMaterial3d(material_handle),
                    Transform::from_xyz(t.translation.x, t.translation.y, t.translation.z),
                Note {
                    key_num: k.key_num,
                    oct: k.oct,
                    length: 1.0,
                    delta_x: 0.0,
                    is_creating: true,
                },
                NoFrustumCulling,
            ));
        }
    }
}

fn update_pressed_note(mut commands: Commands, mut query: Query<(Entity, &mut Transform, &mut Note, &mut Mesh3d)>, query2: Query<&mut Key, With<PressedKey>>, mut query3: Query<&mut Key, Without<PressedKey>>, mut meshes: ResMut<Assets<Mesh>>, mut selected: ResMut<SelectedColor>)
{
    let count = query.iter().count();
    let mut despawn_count = 0;
    for (entity, mut t, mut note, mesh_handle) in &mut query {
        if query2.iter().any(|key| key.key_num == note.key_num && note.is_creating) {
            note.length += 0.04;
            let new_cuboid = Cuboid::new(note.length, 0.1, 0.1); // 新しいサイズ
            let new_mesh = Mesh::from(new_cuboid);
            meshes.insert(mesh_handle.id(), new_mesh); // 既存のメッシュを上書き
            t.translation.x -= 0.02;
            continue;
        }
        query3.iter_mut().for_each(|mut key| {
            key.is_note_created = false;
        });
        note.is_creating = false;
        // Noteの位置が画面外に出たら削除
        if note.delta_x > 30.0 {
            commands.entity(entity).despawn();
            despawn_count += 1;
            println!("Note despawned: {}", count - despawn_count);
        }
        t.translation.x -= 0.04;
        note.delta_x += 0.04;

    }
}

fn display_release(mut query: Query<(&mut Transform, &Key), Without<PressedKey>>) {
    for (mut t, k) in &mut query {
        t.translation.y = k.y_reset;
    }
}

// キー入力を処理するシステム
fn key_input_system(mut commands: Commands, key_input: Res<ButtonInput<KeyCode>>, mut query: Query<(Entity, &mut Transform), With<Camera3d>>, notes: Query<Entity, With<Note>>, mut state: ResMut<WindowState>) {
    for (entity, mut transform) in &mut query {
        let up = key_input.pressed(KeyCode::ArrowUp) || key_input.pressed(KeyCode::KeyW);
        let down = key_input.pressed(KeyCode::ArrowDown) || key_input.pressed(KeyCode::KeyS);
        let left = key_input.pressed(KeyCode::ArrowLeft) || key_input.pressed(KeyCode::KeyA);
        let right = key_input.pressed(KeyCode::ArrowRight) || key_input.pressed(KeyCode::KeyD);
        let digit0 = key_input.just_pressed(KeyCode::Digit0);
        let digit1 = key_input.just_pressed(KeyCode::Digit1);
        let digit2 = key_input.just_pressed(KeyCode::Digit2);
        let digit3 = key_input.just_pressed(KeyCode::Digit3);
        let digit4 = key_input.just_pressed(KeyCode::Digit4);
        let digit5 = key_input.just_pressed(KeyCode::Digit5);
        let digit6 = key_input.just_pressed(KeyCode::Digit6);
        let digit7 = key_input.just_pressed(KeyCode::Digit7);
        let digit8 = key_input.just_pressed(KeyCode::Digit8);
        let digit9 = key_input.just_pressed(KeyCode::Digit9);
        let mut digit = 0;
        if digit0 {
            digit = 0;
        } else if digit1 {
            digit = 1;
        } else if digit2 {
            digit = 2;
        } else if digit3 {
            digit = 3;
        } else if digit4 {
            digit = 4;
        } else if digit5 {
            digit = 5;
        } else if digit6 {
            digit = 6;
        } else if digit7 {
            digit = 7;
        } else if digit8 {
            digit = 8;
        } else if digit9 {
            digit = 9;
        }
        let is_digit = digit0 || digit1 || digit2 || digit3 || digit4 || digit5 || digit6 || digit7 || digit8 || digit9;
        let e = key_input.pressed(KeyCode::KeyE);
        let m_just = key_input.just_pressed(KeyCode::KeyM);
        let q = key_input.pressed(KeyCode::KeyQ);
        let ctrl = key_input.pressed(KeyCode::ControlLeft) || key_input.pressed(KeyCode::ControlRight);
        let shift = key_input.pressed(KeyCode::ShiftLeft) || key_input.pressed(KeyCode::ShiftRight);
        let alt = key_input.pressed(KeyCode::AltLeft) || key_input.pressed(KeyCode::AltRight);
        let esc = key_input.pressed(KeyCode::Escape);
        if up {
            if shift {
                transform.rotation *= Quat::from_rotation_z(0.01);
            } else {
                transform.translation.y += 0.1;
            }
        }
        if down {
            if shift {
                transform.rotation *= Quat::from_rotation_z(-0.01);
            } else {
                transform.translation.y -= 0.1;
            }
        }
        if right {
            if shift {
                transform.rotation *= Quat::from_rotation_x(0.01)
            } else {
                transform.translation.x += 0.1;
            }
        }
        if left {
            if shift {
                transform.rotation *= Quat::from_rotation_x(-0.01);
            } else {
                transform.translation.x -= 0.1;
            }
        }
        if is_digit {
            if ctrl {
                let data = SaveData {
                    transforms: TransformSaveData {
                        translation: transform.translation.to_array(),
                        rotation: transform.rotation.to_array(),
                        scale: transform.scale.to_array(),
                    },
                };
                save_camera_location(&data, digit);
            } else {
                let camera_loc = load_camera_location(digit);
                if (camera_loc.is_some()) {
                    let tf = camera_loc.unwrap().transforms;
                    let ctf = Transform::from_translation(Vec3::from_array(tf.translation)).with_rotation(Quat::from_array(tf.rotation)).with_scale(Vec3::from_array(tf.scale));
                    commands.entity(entity).insert(ctf);
                }
            }
        }
        if ctrl && up || q {
            if shift {
                transform.rotation *= Quat::from_rotation_y(0.01);
            } else {
                transform.translation.z -= 0.1;
            }
        }
        if ctrl && down || e {
            if shift {
                transform.rotation *= Quat::from_rotation_y(-0.01);
            } else {
                transform.translation.z += 0.1;
            }
        }
        if ctrl && shift && alt {
            // 位置をリセット
            transform.translation = CAMERA_POS;
            transform.rotation = Transform::from_xyz(CAMERA_POS.x, CAMERA_POS.y, CAMERA_POS.z).looking_at(Vec3::new(0., 0., -6.3), Vec3::Y).rotation;
        }
        if esc {
            // ノーツをクリア
            notes.iter().for_each(|note| commands.entity(note).despawn());
        }
        if m_just {
            // メニューを開く/閉じる
            state.show_tool_window = !state.show_tool_window;
        }
    }
}


fn handle_midi_input(
    mut commands: Commands,
    mut midi_events: EventReader<MidiData>,
    query: Query<(Entity, &Key)>,
) {
    for data in midi_events.read() {
        let [_, index, _value] = data.message.msg;
        let off = index % 12;
        let oct = index.overflowing_div(12).0;
        let key_str = KEY_RANGE.iter().nth(off.into()).unwrap();

        if data.message.is_note_on() {
            for (entity, key) in query.iter() {
                if key.key_val.eq(&format!("{}{}", key_str, oct).to_string()) {
                    commands.entity(entity).insert(PressedKey);
                }
            }
        } else if data.message.is_note_off() {
            for (entity, key) in query.iter() {
                if key.key_val.eq(&format!("{}{}", key_str, oct).to_string()) {
                    commands.entity(entity).remove::<PressedKey>();
                }
            }
        } else {
        }
    }
}

fn connect_to_first_input_port(input: Res<MidiInput>) {
    if input.is_changed() {
        if let Some((_, port)) = input.ports().get(0) {
            input.connect(port.clone());
        }
    }
}

fn connect_to_first_output_port(input: Res<MidiOutput>) {
    if input.is_changed() {
        if let Some((_, port)) = input.ports().get(0) {
            input.connect(port.clone());
        }
    }
}


fn save_camera_location(data: &SaveData, index: usize) {
    let path = Path::new(".mpv");

    if ! path.exists() {
        if let Err(e) = fs::create_dir_all(path) {
            println!("ディレクトリ作成に失敗しました: {}", e);
            return;
        }
    }
    if let Ok(json) = serde_json::to_string(data) {
        let mut file = File::create(format!(".mpv/save{index}.json")).expect("Failed to create file");
        file.write_all(json.as_bytes()).expect("Failed to write data");
    }    
}

fn load_camera_location(index: usize) -> Option<SaveData> {
    let json = fs::read_to_string(format!(".mpv/save{index}.json")).ok()?;
    serde_json::from_str(&json).ok()
}
