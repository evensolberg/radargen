//! A legend long enough for paper's continuation pages (FR-111), shared
//! by the oracle and the goldens that pin it.

/// 120 items in one sector and ring: two continuation pages, each full.
pub fn long_csv() -> String {
    (0..120).fold("name,sector,ring,description\n".to_owned(), |csv, i| {
        csv + &format!("item {i:03},Tools,Adopt,d\n")
    })
}
