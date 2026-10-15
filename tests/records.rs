// SPDX-License-Identifier: AGPL-3.0-only WITH App-Fair-Distribution-Exception
//! Settings › Records › Reset all records: every game's stored records go, and a saved game that
//! carries its best (Block Blast, Sirtet, 2048) keeps the game but not the record. The prefs
//! store is the machine's own, so each key's previous value is put back afterwards.
#![cfg(feature = "mock")]

/// Every key the reset touches, as the games spell them under gamekit's `save.` prefix.
const KEYS: &[&str] = &[
    "blockblast.best",
    "blockblast.v1",
    "breakout.best",
    "charades.records",
    "mines.records",
    "pacpac.best.pacpac",
    "pacpac.best.mspacpac",
    "pipes.records",
    "sirtet.best",
    "sirtet.v1",
    "solitaire.stats",
    "sudoku.records",
    "twentyfortyeight.best",
    "twentyfortyeight.v1",
];

/// The keys' values before the test, put back on drop even when an assertion fails.
struct Saved(Vec<(String, Option<String>)>);

impl Saved {
    fn take() -> Self {
        Saved(
            KEYS.iter()
                .map(|k| {
                    let key = format!("save.{k}");
                    let previous = day::prefs::get(&key);
                    day::prefs::remove(&key);
                    (key, previous)
                })
                .collect(),
        )
    }
}

impl Drop for Saved {
    fn drop(&mut self) {
        for (key, value) in &self.0 {
            match value {
                Some(v) => {
                    day::prefs::set(key, v);
                }
                None => {
                    day::prefs::remove(key);
                }
            }
        }
    }
}

fn seed(key: &str, value: &str) {
    assert!(day::prefs::set(&format!("save.{key}"), value), "seed {key}");
}

fn stored(key: &str) -> Option<String> {
    day::prefs::get(&format!("save.{key}"))
}

#[test]
fn reset_all_records_clears_every_game() {
    let _saved = Saved::take();
    seed("blockblast.best", "900");
    seed(
        "blockblast.v1",
        r#"{"grid":[-1,-1,-1,-1,-1,-1,-1,-1,2,2,2,-1,7,7,-1,6,-1,-1,-1,-1,-1,-1,-1,6,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1],"tray":[["T_dn",7],null,["T_lt",4]],"score":250,"best":900,"combo":1,"since_clear":0,"difficulty":"Normal","game_over":false}"#,
    );
    seed("breakout.best", "1858");
    seed(
        "charades.records",
        r#"{"best":{"animals":7},"played":{"animals":2}}"#,
    );
    seed(
        "mines.records",
        r#"{"played":[3,0,0],"won":[1,0,0],"best":[24.5,null,null]}"#,
    );
    seed("pacpac.best.pacpac", "3210");
    seed("pacpac.best.mspacpac", "4960");
    seed("pipes.records", r#"{"best":[33,null,160]}"#);
    seed("sirtet.best", "1200");
    seed(
        "sirtet.v1",
        r#"{"grid":[-1,-1,-1,-1,-1,-1,-1,-1,-1,-1],"kind":0,"rot":0,"prow":0,"pcol":3,"bag":[1,2],"next":3,"score":40,"best":1200,"lines":1,"clearing":[]}"#,
    );
    seed(
        "solitaire.stats",
        r#"{"played":[3,0],"won":[1,0],"best_time":[null,null],"best_score":[0,0],"streak":0,"best_streak":1}"#,
    );
    seed("sudoku.records", r#"{"best":[null,120.5,null,null]}"#);
    seed("twentyfortyeight.best", "2048");
    seed(
        "twentyfortyeight.v1",
        r#"{"grid":[[0,0,0,0],[0,0,0,0],[0,0,0,0],[0,0,0,2]],"score":40,"best":2048,"won":false}"#,
    );

    dayapp::reset_all_records();

    for key in [
        "blockblast.best",
        "breakout.best",
        "charades.records",
        "mines.records",
        "pacpac.best.pacpac",
        "pacpac.best.mspacpac",
        "pipes.records",
        "sirtet.best",
        "solitaire.stats",
        "sudoku.records",
        "twentyfortyeight.best",
    ] {
        assert_eq!(stored(key), None, "{key} survived the reset");
    }
    // A saved game in progress stays, with its best brought down: to the score in Block
    // Blast, to zero in Sirtet and 2048, as each game's own Reset High Score leaves it.
    let bb: serde_json::Value =
        serde_json::from_str(&stored("blockblast.v1").expect("blockblast save kept")).unwrap();
    assert_eq!(bb["best"], 250);
    assert_eq!(bb["score"], 250);
    let st: serde_json::Value =
        serde_json::from_str(&stored("sirtet.v1").expect("sirtet save kept")).unwrap();
    assert_eq!(st["best"], 0);
    assert_eq!(st["score"], 40);
    let tf: serde_json::Value =
        serde_json::from_str(&stored("twentyfortyeight.v1").expect("2048 save kept")).unwrap();
    assert_eq!(tf["best"], 0);
    assert_eq!(tf["score"], 40);
}
