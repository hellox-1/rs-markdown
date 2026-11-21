mod cli;
mod html;
mod images;
mod markdown;
mod syntax;
mod table;
mod viewer;

fn main() -> anyhow::Result<()> {
    cli::run()
}
