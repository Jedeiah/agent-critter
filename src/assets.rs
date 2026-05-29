use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "assets/themes/"]
pub struct ThemeAssets;
