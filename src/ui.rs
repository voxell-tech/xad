use bevy::ecs::hierarchy::ChildSpawnerCommands;
use bevy::prelude::*;

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_ui);
    }
}

const PANEL_BG: Color = Color::srgb(0.09, 0.09, 0.11);
const PART_SELECTOR_BG: Color = Color::srgb(0.13, 0.13, 0.16);
const BORDER: Color = Color::srgb(0.22, 0.22, 0.26);
const TEXT: Color = Color::srgb(0.86, 0.86, 0.88);
const TEXT_DIM: Color = Color::srgb(0.42, 0.42, 0.47);
const TEXT_BLUE: Color = Color::srgb(0.45, 0.65, 1.00);
const TEXT_GREEN: Color = Color::srgb(0.40, 0.92, 0.52);
const TEXT_RED: Color = Color::srgb(1.00, 0.42, 0.42);
const TEXT_YELLOW: Color = Color::srgb(0.92, 0.78, 0.28);
const ISLAND_BG: Color = Color::srgb(0.14, 0.14, 0.18);
const ISLAND_BORDER: Color = Color::srgb(0.28, 0.28, 0.34);
const ICON_BTN_BG: Color = Color::srgb(0.18, 0.18, 0.22);
const ICON_BTN_BORDER: Color = Color::srgb(0.25, 0.25, 0.30);

const PANEL_W: f32 = 240.0;

fn setup_ui(mut commands: Commands) {
    commands
        .spawn(Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            flex_direction: FlexDirection::Column,
            position_type: PositionType::Absolute,
            ..default()
        })
        .with_children(|root| {
            // Content row: feature tree + scene area
            root.spawn(Node {
                width: Val::Percent(100.0),
                flex_grow: 1.0,
                flex_direction: FlexDirection::Row,
                ..default()
            })
            .with_children(|row| {
                spawn_feature_tree(row);
                row.spawn(Node {
                    flex_grow: 1.0,
                    height: Val::Percent(100.0),
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    ..default()
                })
                .with_children(|scene| {
                    spawn_toolbar_island(scene);
                });
            });
        });
}

fn spawn_toolbar_island(parent: &mut ChildSpawnerCommands<'_>) {
    const GROUPS: &[&[(&str, &str)]] = &[
        &[("SKT", "New Sketch"), ("FIN", "Finish Sketch")],
        &[
            ("EXT", "Extrude"),
            ("REV", "Revolve"),
            ("SWP", "Sweep"),
            ("LFT", "Loft"),
            ("HLE", "Hole"),
            ("THD", "Thread"),
        ],
        &[
            ("FLT", "Fillet"),
            ("CHM", "Chamfer"),
            ("SHL", "Shell"),
            ("DFT", "Draft"),
            ("CMB", "Combine"),
        ],
        &[("JNT", "Joint"), ("ALN", "Align"), ("RGD", "Rigid Group")],
        &[("PLN", "Offset Plane"), ("AXS", "Axis"), ("PNT", "Point")],
        &[("MSR", "Measure"), ("SEC", "Section")],
    ];

    parent
        .spawn((
            Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                padding: UiRect::all(Val::Px(6.0)),
                margin: UiRect::top(Val::Px(12.0)),
                border: UiRect::all(Val::Px(1.0)),
                border_radius: BorderRadius::all(Val::Px(10.0)),
                ..default()
            },
            BackgroundColor(ISLAND_BG),
            BorderColor::from(ISLAND_BORDER),
        ))
        .with_children(|island| {
            for (i, group) in GROUPS.iter().enumerate() {
                if i > 0 {
                    island.spawn((
                        Node {
                            width: Val::Px(1.0),
                            height: Val::Px(24.0),
                            margin: UiRect::horizontal(Val::Px(4.0)),
                            ..default()
                        },
                        BackgroundColor(BORDER),
                    ));
                }

                for &(abbr, _) in *group {
                    island
                        .spawn((
                            Button,
                            Node {
                                width: Val::Px(34.0),
                                height: Val::Px(34.0),
                                align_items: AlignItems::Center,
                                justify_content:
                                    JustifyContent::Center,
                                margin: UiRect::horizontal(Val::Px(
                                    2.0,
                                )),
                                border: UiRect::all(Val::Px(1.0)),
                                border_radius: BorderRadius::all(
                                    Val::Px(5.0),
                                ),
                                ..default()
                            },
                            BackgroundColor(ICON_BTN_BG),
                            BorderColor::from(ICON_BTN_BORDER),
                        ))
                        .with_children(|btn| {
                            btn.spawn((
                                Text::new(abbr),
                                TextFont {
                                    font_size: 10.0,
                                    ..default()
                                },
                                TextColor(TEXT),
                            ));
                        });
                }
            }
        });
}

fn spawn_feature_tree(parent: &mut ChildSpawnerCommands<'_>) {
    parent
        .spawn((
            Node {
                width: Val::Px(PANEL_W),
                flex_shrink: 0.0,
                flex_direction: FlexDirection::Column,
                border: UiRect::all(Val::Px(1.0)),
                border_radius: BorderRadius::all(Val::Px(12.0)),
                margin: UiRect::all(Val::Px(10.0)),
                overflow: Overflow::hidden(),
                ..default()
            },
            BackgroundColor(PANEL_BG),
            BorderColor::from(BORDER),
        ))
        .with_children(|panel| {
            panel
                .spawn((
                    Button,
                    Node {
                        flex_direction: FlexDirection::Row,
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::SpaceBetween,
                        padding: UiRect::axes(
                            Val::Px(12.0),
                            Val::Px(10.0),
                        ),
                        margin: UiRect::all(Val::Px(10.0)),
                        border: UiRect::all(Val::Px(1.0)),
                        border_radius: BorderRadius::all(Val::Px(
                            7.0,
                        )),
                        ..default()
                    },
                    BackgroundColor(PART_SELECTOR_BG),
                    BorderColor::from(BORDER),
                ))
                .with_children(|sel| {
                    sel.spawn((
                        Text::new("Part 1"),
                        TextFont {
                            font_size: 13.0,
                            ..default()
                        },
                        TextColor(TEXT),
                    ));
                    sel.spawn((
                        Text::new("v"),
                        TextFont {
                            font_size: 11.0,
                            ..default()
                        },
                        TextColor(TEXT_DIM),
                    ));
                });

            panel
                .spawn(Node {
                    padding: UiRect {
                        left: Val::Px(12.0),
                        right: Val::Px(12.0),
                        top: Val::Px(2.0),
                        bottom: Val::Px(6.0),
                    },
                    ..default()
                })
                .with_children(|h| {
                    h.spawn((
                        Text::new("features"),
                        TextFont {
                            font_size: 10.5,
                            ..default()
                        },
                        TextColor(TEXT_DIM),
                    ));
                });

            panel
                .spawn(Node {
                    width: Val::Percent(100.0),
                    flex_direction: FlexDirection::Column,
                    flex_grow: 1.0,
                    overflow: Overflow::clip_y(),
                    ..default()
                })
                .with_children(|tree| {
                    tree_item(tree, "v Origin", 0, TEXT, 12.5);
                    tree_item(tree, "X Axis", 1, TEXT_RED, 12.0);
                    tree_item(tree, "Y Axis", 1, TEXT_GREEN, 12.0);
                    tree_item(tree, "Z Axis", 1, TEXT_BLUE, 12.0);

                    tree_item(tree, "v Femur", 0, TEXT, 12.5);
                    tree_item(tree, "Sketch 1", 1, TEXT_YELLOW, 12.0);
                    tree_item(tree, "Extrude 1", 1, TEXT, 12.0);
                    tree_item(tree, "Extrude 2", 1, TEXT, 12.0);

                    tree_item(tree, "v Tibia", 0, TEXT, 12.5);
                    tree_item(tree, "Sketch 2", 1, TEXT_YELLOW, 12.0);
                    tree_item(tree, "Fillet 1", 1, TEXT, 12.0);
                    tree_item(tree, "Chamfer 1", 1, TEXT, 12.0);
                    tree_item(tree, "Mirror 1", 1, TEXT, 12.0);
                    tree_item(tree, "Pattern 1", 1, TEXT, 12.0);
                });

            panel.spawn((
                Node {
                    width: Val::Percent(100.0),
                    height: Val::Px(1.0),
                    ..default()
                },
                BackgroundColor(BORDER),
            ));

            panel
                .spawn(Node {
                    padding: UiRect {
                        left: Val::Px(12.0),
                        right: Val::Px(12.0),
                        top: Val::Px(10.0),
                        bottom: Val::Px(4.0),
                    },
                    ..default()
                })
                .with_children(|h| {
                    h.spawn((
                        Text::new("parts"),
                        TextFont {
                            font_size: 10.5,
                            ..default()
                        },
                        TextColor(TEXT_DIM),
                    ));
                });

            panel.spawn(Node {
                width: Val::Percent(100.0),
                min_height: Val::Px(60.0),
                flex_direction: FlexDirection::Column,
                ..default()
            });
        });
}

fn tree_item(
    parent: &mut ChildSpawnerCommands<'_>,
    label: &str,
    indent: u32,
    color: Color,
    font_size: f32,
) {
    parent
        .spawn((
            Button,
            Node {
                width: Val::Percent(100.0),
                padding: UiRect {
                    left: Val::Px(8.0 + indent as f32 * 14.0),
                    right: Val::Px(8.0),
                    top: Val::Px(3.5),
                    bottom: Val::Px(3.5),
                },
                ..default()
            },
            BackgroundColor(Color::NONE),
        ))
        .with_children(|row| {
            row.spawn((
                Text::new(label),
                TextFont {
                    font_size,
                    ..default()
                },
                TextColor(color),
            ));
        });
}
