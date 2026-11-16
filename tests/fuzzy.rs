//! Fuzzy filter matching: typos and missing letters must still find a title.

use anirel::app::fuzzy::matches;

#[test]
fn a_missing_letter_still_matches() {
    assert!(matches("Yuru Camp", "yurcamp"));
}

#[test]
fn a_wrong_letter_still_matches() {
    assert!(matches("Yuru Camp", "yorucamp"));
}

#[test]
fn spacing_and_punctuation_are_ignored() {
    assert!(matches("Yuru Camp△", "yurucamp"));
    assert!(matches("YURU CAMP", "yurucamp"));
}

#[test]
fn a_word_inside_a_longer_title_matches() {
    assert!(matches("Yuru Camp Season 2", "camp"));
    assert!(matches("Yuru Camp Season 2", "yorucamp"));
}

#[test]
fn short_filters_stay_precise() {
    assert!(!matches("Yuru Camp", "xo"));
    assert!(matches("Yuru Camp", "yu"));
}

#[test]
fn unrelated_titles_do_not_match() {
    assert!(!matches("Yuru Camp", "naruto"));
    assert!(!matches("Yuru Camp", "hibike"));
}

#[test]
fn an_empty_filter_matches_anything() {
    assert!(matches("Yuru Camp", ""));
}
