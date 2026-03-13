use chip_8::ui::App;

fn main() -> std::io::Result<()> {
    ratatui::run(|terminal| App::default().run(terminal))
}
