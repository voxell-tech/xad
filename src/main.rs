use bevy::app::App;
use xad::XadPlugin;

fn main() {
    let mut app = App::new();
    app.add_plugins(XadPlugin);
    app.run();
}
