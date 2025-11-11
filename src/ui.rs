use bevy::color::palettes::tailwind;
use bevy::ecs::spawn::{SpawnRelatedBundle, SpawnableList};
use bevy::prelude::*;

pub fn button(text: impl Into<String>) -> impl Bundle {
    (
        Node {
            // width: Val::Px(100.0),
            // height: Val::Px(40.0),
            padding: UiRect::all(Val::Px(10.0)),
            margin: UiRect::all(Val::Px(4.0)),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            ..default()
        },
        BorderRadius::all(Val::Px(8.0)),
        BackgroundColor(tailwind::GRAY_100.into()),
        Button,
        Children::spawn(Spawn((
            Text::new(text),
            TextLayout::default().with_justify(JustifyText::Center),
            TextColor(Color::BLACK),
        ))),
    )
}

pub fn row(
    list: impl SpawnableList<ChildOf>,
) -> (
    Node,
    SpawnRelatedBundle<ChildOf, impl SpawnableList<ChildOf>>,
) {
    (
        Node {
            flex_direction: FlexDirection::Row,
            ..default()
        },
        Children::spawn(list),
    )
}

pub fn col(
    list: impl SpawnableList<ChildOf>,
) -> (
    Node,
    SpawnRelatedBundle<ChildOf, impl SpawnableList<ChildOf>>,
) {
    (
        Node {
            flex_direction: FlexDirection::Column,
            ..default()
        },
        Children::spawn(list),
    )
}
