//! Tests for the MyAnimeList web parsing helpers.

use anirel::domain::relation::RelationKind;
use anirel::infra::mal_web::{html_unescape, parse_related_entries};

const RELATED_SECTION: &str = r#"
<div class="related-entries">
  <div class="entries-tile">
    <div class="entry borderClass ">
      <div class="image">
        <a href="https://myanimelist.net/anime/38474/Yuru_Camp_Season_2"><img alt="Yuru Camp"></a>
      </div>
      <div class="content">
        <div class="relation">
          Sequel
          (TV)
        </div>
        <div class="title">
          <a href="https://myanimelist.net/anime/38474/Yuru_Camp_Season_2">
            Yuru Camp△ Season 2
          </a>
        </div>
      </div>
    </div>
  </div>
  <table class="entries-table">
    <tr class="">
      <td valign="top" class="ar fw-n borderClass nowrap">
        Side Story:
      </td>
      <td width="100%" class="borderClass">
        <ul class="entries">
          <li class=""><a href="https://myanimelist.net/anime/37341/Yuru_Camp_Specials">Yuru Camp△ Specials</a> (Special)</li>
          <li class=""><a href="https://myanimelist.net/anime/38475/Yuru_Camp_Movie">Yuru Camp△ Movie</a> (Movie)</li>
        </ul>
      </td>
    </tr>
    <tr class="">
      <td valign="top" class="ar fw-n borderClass nowrap">
        Adaptation:
      </td>
      <td width="100%" class="borderClass">
        <ul class="entries">
          <li class=""><a href="https://myanimelist.net/manga/94376/Yuru_Camp">Yuru Camp△</a> (Manga)</li>
        </ul>
      </td>
    </tr>
  </table>
</div>
"#;

#[test]
fn html_entities_are_decoded() {
    assert_eq!(html_unescape("a &quot;b&quot;"), "a \"b\"");
    assert_eq!(html_unescape("Tom &amp; Jerry"), "Tom & Jerry");
    assert_eq!(html_unescape("&#039;quoted&#039;"), "'quoted'");
    assert_eq!(html_unescape("plain"), "plain");
}

#[test]
fn related_entries_are_parsed_from_tiles_and_tables() {
    let html = format!(
        "<html><body><h2 id=\"related_entries\">Related Entries</h2>{RELATED_SECTION}<h2>Statistics</h2></body></html>"
    );
    let relations = parse_related_entries(&html);

    assert!(
        relations.contains(&(38474, RelationKind::Sequel)),
        "the tile entry is found: {relations:?}"
    );
    assert!(
        relations.contains(&(37341, RelationKind::SideStory)),
        "the table entry is found: {relations:?}"
    );
    assert!(
        relations.contains(&(38475, RelationKind::SideStory)),
        "the second table entry is found: {relations:?}"
    );
    assert!(
        !relations.iter().any(|(id, _)| *id == 94376),
        "manga adaptations are not anime relations"
    );
}

#[test]
fn a_page_without_related_entries_yields_nothing() {
    assert!(parse_related_entries("<html><body>nothing here</body></html>").is_empty());
}
