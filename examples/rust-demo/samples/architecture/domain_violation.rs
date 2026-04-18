use crate::infra::db;

pub fn load_order() {
    let _ = db::connect();
}
