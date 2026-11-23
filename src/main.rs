mod cli;
mod html;
mod images;
mod markdown;
mod syntax;
mod table;
mod theme;
mod viewer;

fn main() -> anyhow::Result<()> {
    cli::run()
}
