/*
 *  Copyright 2022, The Cozo Project Authors.
 *
 *  This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0.
 *  If a copy of the MPL was not distributed with this file,
 *  You can obtain one at https://mozilla.org/MPL/2.0/.
 *
 */
#![feature(test)]

extern crate test;

use cozo::{DbInstance, NamedRows};
use itertools::Itertools;
use lazy_static::{initialize, lazy_static};
use serde_json::json;
use std::collections::BTreeMap;
use test::Bencher;

fn insert_data(db: &DbInstance) {
    let mut to_import = BTreeMap::new();
    to_import.insert(
        "plain".to_string(),
        NamedRows {
            headers: vec!["k".to_string(), "v".to_string()],
            rows: (0..10000).map(|i| vec![json!(i), json!(i)]).collect_vec(),
        },
    );

    to_import.insert(
        "tt1".to_string(),
        NamedRows {
            headers: vec!["k".to_string(), "vld".to_string(), "v".to_string()],
            rows: (0..10000)
                .map(|i| vec![json!(i), json!([0, true]), json!(i)])
                .collect_vec(),
        },
    );

    to_import.insert(
        "tt100".to_string(),
        NamedRows {
            headers: vec!["k".to_string(), "vld".to_string(), "v".to_string()],
            rows: (0..10000)
                .flat_map(|i| (0..100).map(move |vld| vec![json!(i), json!([vld, true]), json!(i)]))
                .collect_vec(),
        },
    );

    to_import.insert(
        "tt10000".to_string(),
        NamedRows {
            headers: vec!["k".to_string(), "vld".to_string(), "v".to_string()],
            rows: (0..10000)
                .flat_map(|i| {
                    (0..10000).map(move |vld| vec![json!(i), json!([vld, true]), json!(i)])
                })
                .collect_vec(),
        },
    );

    db.import_relations(to_import).unwrap();
}

lazy_static! {
    static ref TEST_DB: DbInstance = {
        let db_path = "_time_travel_rocks.db";
        let db = DbInstance::new("rocksdb", db_path, "").unwrap();

        let create_res = db.run_script(
            r#"
        {:create plain {k: Int => v}}
        {:create tt1 {k: Int, vld: Validity => v}}
        {:create tt100 {k: Int, vld: Validity => v}}
        {:create tt10000 {k: Int, vld: Validity => v}}
        "#,
            Default::default(),
        );

        if create_res.is_ok() {
            insert_data(&db);
        } else {
            println!("database already exists, skip import");
        }

        db
    };
}

#[bench]
fn time_travel_init(_: &mut Bencher) {
    initialize(&TEST_DB);
}