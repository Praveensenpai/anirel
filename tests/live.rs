//! Live network checks. Run with `cargo test -- --ignored --nocapture`.

use anirel::infra::anilist::AniListClient;

#[tokio::test]
#[ignore]
async fn resolves_a_batch_of_real_ids() {
    let client = AniListClient::new();
    let ids: Vec<i64> = (20..40).collect();
    let results = client.resolve(&ids).await.expect("resolve failed");
    let found = results.iter().flatten().count();
    println!("requested {} ids, resolved {found}", ids.len());
    for (id, result) in ids.iter().zip(results.iter()) {
        if let Some(media) = result {
            println!(
                "  {id} -> {} ({}), {} relations",
                media.title.romaji.clone().unwrap_or_default(),
                media.format.clone().unwrap_or_default(),
                media.relations.as_ref().map(|r| r.edges.len()).unwrap_or(0)
            );
        }
    }
    assert!(found > 0, "nothing resolved");
}

#[tokio::test]
#[ignore]
async fn resolves_a_single_real_id() {
    let client = AniListClient::new();
    let results = client.resolve(&[34798]).await.expect("resolve failed");
    let media = results.into_iter().flatten().next().expect("nothing");
    println!("title: {:?}", media.title);
    println!(
        "relations: {}",
        media.relations.map(|r| r.edges.len()).unwrap_or(0)
    );
}

#[tokio::test]
#[ignore]
async fn fetches_and_decodes_a_cover() {
    let url = "https://cdn.myanimelist.net/r/192x272/images/anime/1160/160129.jpg?s=9dd558a657afaa6407b79068e0ef7e6e";
    let path = anirel::art::ensure_cached(url)
        .await
        .expect("cover was not cached");
    println!("cached at {}", path.display());
    let art = anirel::art::decode(&path).expect("decode failed");
    println!("decoded {}x{}", art.width, art.height);
    assert!(art.width > 10 && art.height > 10);
}
