use music_player_backend::database::{create_in_memory_pool, repositories::{track_repo::{SqliteTrackRepository, TrackRepository}, album_repo::{SqliteAlbumRepository, AlbumRepository}}};

#[tokio::test]
async fn pages_are_complete_stable_local_and_account_scoped() {
    let pool = create_in_memory_pool().await.unwrap();
    let tracks = SqliteTrackRepository::new(pool.clone());
    let albums = SqliteAlbumRepository::new(pool.clone());
    assert_eq!(tracks.library_page("default", 0, 200).await.unwrap().1, 0);
    assert_eq!(albums.library_page(0, 100).await.unwrap().1, 0);
    for i in 0..201 {
        sqlx::query("INSERT INTO albums(id,title,normalized_title,created_at) VALUES(?,'Same','same',1)").bind(format!("a{i:04}")).execute(&pool).await.unwrap();
    }
    for i in 0..1036 {
        sqlx::query("INSERT INTO tracks(id,file_path,file_size,modified_timestamp,title,normalized_title,duration_secs,format,album_id,created_at,updated_at) VALUES(?,?,1,1,'Same','same',1,'wav',?,1,1)")
            .bind(format!("t{i:04}")).bind(format!("/fixture/{i}.wav")).bind(format!("a{:04}",i%201)).execute(&pool).await.unwrap();
    }
    sqlx::query("INSERT INTO tracks(id,file_path,file_size,modified_timestamp,title,normalized_title,duration_secs,format,created_at,updated_at) VALUES('remote','online://remote',1,1,'Same','same',1,'online',1,1)").execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO track_statistics(user_id,track_id,manual_like) VALUES('default','t0000',1)").execute(&pool).await.unwrap();
    let mut ids=Vec::new();
    for offset in (0..1200).step_by(200) {
        let (page,total)=tracks.library_page("default",offset,200).await.unwrap();
        assert_eq!(total,1036);
        ids.extend(page.into_iter().map(|t|t.id));
    }
    assert_eq!(ids.len(),1036);
    assert!(ids.windows(2).all(|pair|pair[0]<pair[1]));
    assert_eq!(tracks.library_page("default",0,200).await.unwrap().0[0].manual_like,1);
    assert_eq!(tracks.library_page("other",0,200).await.unwrap().0[0].manual_like,0);
    let mut ids=Vec::new();
    for offset in [0,100,200,300] {
        let (page,total)=albums.library_page(offset,100).await.unwrap();assert_eq!(total,201);ids.extend(page.into_iter().map(|a|a.id));
    }
    assert_eq!(ids.len(),201);assert!(ids.windows(2).all(|pair|pair[0]<pair[1]));
    assert!(tracks.library_page("default",1036,200).await.unwrap().0.is_empty());
    sqlx::query("DELETE FROM tracks WHERE id >= 't0200'").execute(&pool).await.unwrap();
    let (page,total)=tracks.library_page("default",200,200).await.unwrap();
    assert_eq!(total,200);assert!(page.is_empty());
    let (page,total)=albums.library_page(200,100).await.unwrap();
    assert_eq!(total,200);assert!(page.is_empty());
}
