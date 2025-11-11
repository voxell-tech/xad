use bevy::prelude::*;
use bevy_vello::VelloPlugin;
use bevy_vello::prelude::*;
use bevy_vello::vello::peniko::Brush;
use bevy_vello::vello::peniko::Fill;
use fiksi::ElementValue;
use fiksi::constraints::{LineCircleTangency, PointCircleIncidence};
use fiksi::elements::Line;
use fiksi::elements::{Circle, Length, Point};
use fiksi::kurbo::Affine;
use fiksi::{SolvingOptions, System};

use crate::ui::*;

mod ui;

fn main() {
    let mut app = App::new();

    app.add_plugins((DefaultPlugins, VelloPlugin::default()));

    app.init_resource::<FiksiSystem>();

    app.add_systems(PreStartup, setup)
        .add_systems(Startup, (toolbar, add_circle))
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
            Spawn(button("Select")),
            Spawn(button("Point")),
            Spawn(button("Line")),
            Spawn(button("Circle")),
        )))),
    ));
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
