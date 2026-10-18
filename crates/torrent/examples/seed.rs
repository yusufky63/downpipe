//! A seeder on loopback for trying torrents by hand without the internet:
//! `cargo run -p downpipe-torrent --features testing --example seed`.
//! Prints a magnet link that names this seeder as its peer and the path of
//! the `.torrent` file, then seeds until Enter is pressed. Start the app with
//! `DOWNPIPE_TORRENT_BIND=127.0.0.1` and DHT off to keep it all local.

use downpipe_torrent::testing::{LocalSeeder, content};

#[tokio::main]
async fn main() {
    let files = [
        ("Sample Film.mkv", content(24 * 1024 * 1024, 1)),
        ("Extras/Making of.mp4", content(6 * 1024 * 1024, 2)),
        ("Extras/Poster.jpg", content(700 * 1024, 3)),
        ("Subtitles/English.srt", content(48 * 1024, 4)),
        ("Read me.txt", content(2 * 1024, 5)),
    ];
    // 2 MiB/s, so progress can be watched.
    let seeder = LocalSeeder::start("Sample Pack", &files, 2 * 1024 * 1024).await;
    let out = std::env::temp_dir().join("downpipe-sample.torrent");
    std::fs::write(&out, &seeder.torrent).expect("the .torrent written");
    println!("magnet: {}", seeder.magnet());
    println!("torrent: {}", out.display());
    println!("Seeding on {}. Press Enter to stop.", seeder.addr);
    let _ = tokio::task::spawn_blocking(|| std::io::stdin().read_line(&mut String::new())).await;
    seeder.stop().await;
}
