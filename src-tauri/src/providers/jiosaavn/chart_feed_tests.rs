use super::*;
#[test]
fn merges_international_and_regional_editorial_lists_with_real_artwork() {
    let charts = merge_charts(&[
        serde_json::json!([
            {"id":"12","title":"Global Hits","image":"https://example.test/global-150x150.jpg"},
            {"listid":"13","listname":"English Top 50"}
        ]),
        serde_json::json!([
            {"id":12,"title":"Duplicate"},{"id":"14","title":"India Top 50"},
            {"id":"https://untrusted.test","title":"Invalid"}
        ]),
    ]);
    assert_eq!(charts.len(), 3);
    assert_eq!(charts[0]["title"], "Global Hits");
    assert_eq!(
        charts[0]["image"],
        "https://example.test/global-500x500.jpg"
    );
    assert_eq!(charts[2]["title"], "India Top 50");
}
