use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use bevy_vello::VelloPlugin;
use bevy_vello::prelude::*;
use bevy_vello::vello::peniko::Brush;
use bevy_vello::vello::peniko::Fill;
use fiksi::ElementValue;
use fiksi::constraints::{LineCircleTangency, PointCircleIncidence};
use fiksi::elements::Line;
use fiksi::elements::Point as FiksiPoint;
use fiksi::elements::{Circle, Length, Point};
use fiksi::kurbo::Affine;
use fiksi::{SolvingOptions, System};

use crate::ui::*;

mod ui;

fn main() {
    let mut app = App::new();

    app.add_plugins((DefaultPlugins, VelloPlugin::default()));

    app.init_resource::<FiksiSystem>()
        .init_resource::<CadMode>()
        .init_resource::<WorldCursor>()
        .init_resource::<LineToolState>();

    app.add_systems(PreStartup, setup)
        .add_systems(Startup, (toolbar, add_circle))
        .add_systems(
            Update,
            (
                update_mode_selection,
                sync_mode_button_visuals,
                update_world_cursor,
                point_tool_clicks,
                line_tool_clicks,
            ),
        )
        .add_systems(
            PostUpdate,
            (solve, render.run_if(resource_changed::<FiksiSystem>)).chain(),
        );

    app.run();
}

#[derive(Resource, Default, Deref, DerefMut)]
pub struct FiksiSystem(pub System);

fn setup(mut commands: Commands) {
    commands.spawn((Camera2d, VelloView));
    commands.spawn(VelloScene::default());
}

fn solve(mut system: ResMut<FiksiSystem>, key_input: Res<ButtonInput<KeyCode>>) {
    if key_input.just_pressed(KeyCode::Space) {
        system.solve(SolvingOptions::DEFAULT);
    }
}

fn toolbar(mut commands: Commands) {
    commands.spawn((
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            padding: UiRect::all(Val::Vh(8.0)),
            flex_direction: FlexDirection::Row,
            ..default()
        },
        Children::spawn(Spawn(col((
            Spawn((
                button("Select"),
                ModeButton {
                    mode: CadMode::Select,
                },
            )),
            Spawn((
                button("Point"),
                ModeButton {
                    mode: CadMode::Point,
                },
            )),
            Spawn((
                button("Line"),
                ModeButton {
                    mode: CadMode::Line,
                },
            )),
            Spawn((
                button("Circle"),
                ModeButton {
                    mode: CadMode::Circle,
                },
            )),
        )))),
    ));
}

/// Update the current CAD mode based on button presses.
fn update_mode_selection(
    mut q: Query<(&ModeButton, &Interaction), (Changed<Interaction>, With<Button>)>,
    mut current_mode: ResMut<CadMode>,
) {
    for (mode_button, interaction) in &mut q {
        if *interaction == Interaction::Pressed {
            *current_mode = mode_button.mode;
        }
    }
}

/// Temporarily change button visuals to indicate the current mode.
fn sync_mode_button_visuals(
    current_mode: Res<CadMode>,
    mut q: Query<(&ModeButton, &mut BackgroundColor)>,
) {
    if !current_mode.is_changed() {
        return;
    }

    for (mode_button, mut bg) in &mut q {
        if mode_button.mode == *current_mode {
            bg.0 = Color::srgb(0.25, 0.5, 0.9);
        } else {
            bg.0 = Color::srgb(0.15, 0.15, 0.15);
        }
    }
}

fn update_world_cursor(
    windows: Query<&Window, With<PrimaryWindow>>,
    camera_q: Query<(&Camera, &GlobalTransform), With<Camera2d>>,
    mut cursor: ResMut<WorldCursor>,
) {
    let Ok(window) = windows.single() else {
        cursor.in_window = false;
        return;
    };

    let Ok((camera, cam_transform)) = camera_q.single() else {
        cursor.in_window = false;
        return;
    };

    let Some(screen_pos) = window.cursor_position() else {
        cursor.in_window = false;
        return;
    };

    let Ok(world_pos) = camera.viewport_to_world_2d(cam_transform, screen_pos) else {
        cursor.in_window = false;
        return;
    };

    cursor.position = world_pos;
    cursor.in_window = true;
}

fn add_circle(mut system: ResMut<FiksiSystem>) {
    let s = &mut system;

    let p = Point::create(s, 30., -10.);
    let r = Length::create(s, 50.0);
    let circle = Circle::create(s, p, r);

    let p0 = Point::create(s, -100.0, 0.0);
    let p1 = Point::create(s, 0.0, 100.0);
    let line = Line::create(s, p0, p1);

    LineCircleTangency::create(s, line, circle);
    PointCircleIncidence::create(s, p0, circle);
}

fn point_tool_clicks(
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    cursor: Res<WorldCursor>,
    mut system: ResMut<FiksiSystem>,
    mode: Res<CadMode>,
    ui_buttons: Query<&Interaction, With<Button>>,
) {
    if *mode != CadMode::Point {
        return;
    }

    if !mouse_buttons.just_pressed(MouseButton::Left) {
        return;
    }

    if ui_buttons
        .iter()
        .any(|interaction| *interaction != Interaction::None)
    {
        return;
    }

    if !cursor.in_window {
        return;
    }

    let pos = cursor.position;
    let s = &mut *system;

    FiksiPoint::create(s, pos.x as f64, -pos.y as f64);
}

fn line_tool_clicks(
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    cursor: Res<WorldCursor>,
    mut system: ResMut<FiksiSystem>,
    mode: Res<CadMode>,
    mut line_state: ResMut<LineToolState>,
    ui_buttons: Query<&Interaction, With<Button>>,
) {
    if *mode != CadMode::Line {
        return;
    }

    if !mouse_buttons.just_pressed(MouseButton::Left) {
        return;
    }

    if ui_buttons
        .iter()
        .any(|interaction| *interaction != Interaction::None)
    {
        return;
    }

    if !cursor.in_window {
        return;
    }

    let pos = cursor.position;
    let s = &mut *system;

    match line_state.start_pos {
        None => {
            line_state.start_pos = Some(pos);
        }
        Some(start) => {
            let p0 = FiksiPoint::create(s, start.x as f64, -start.y as f64);
            let p1 = FiksiPoint::create(s, pos.x as f64, -pos.y as f64);
            Line::create(s, p0, p1);

            line_state.start_pos = None;
        }
    }
}

fn render(mut q_scenes: Query<&mut VelloScene>, system: Res<FiksiSystem>) -> Result {
    let stroke = kurbo::Stroke::new(2.0);
    let white_brush = Brush::Solid(peniko::Color::WHITE);
    // let black_brush = Brush::Solid(peniko::Color::BLACK);

    let mut scene = q_scenes.single_mut()?;
    *scene = vello::Scene::new().into();

    for el in system.get_element_handles() {
        let val = el.get_value(&system);
        match val {
            // fiksi::ElementValue::Length(_) => todo!(),
            ElementValue::Point(point) => scene.fill(
                Fill::NonZero,
                Affine::IDENTITY,
                &white_brush,
                None,
                &kurbo::Rect::from_center_size(point, (4.0, 4.0)),
            ),
            ElementValue::Line(line) => {
                scene.stroke(&stroke, Affine::IDENTITY, &white_brush, None, &line)
            }
            ElementValue::Circle(circle) => {
                scene.stroke(&stroke, Affine::IDENTITY, &white_brush, None, &circle)
            }
            _ => {}
        }
    }

    Ok(())
}

/// Indicate which tool is currently active.
#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq)]
pub enum CadMode {
    Select,
    Point,
    Line,
    Circle,
}

impl Default for CadMode {
    fn default() -> Self {
        CadMode::Select
    }
}

#[derive(Component)]
struct ModeButton {
    mode: CadMode,
}

#[derive(Resource, Default)]
struct WorldCursor {
    position: Vec2,
    in_window: bool,
}

#[derive(Resource, Default)]
struct LineToolState {
    start_pos: Option<Vec2>,
}
