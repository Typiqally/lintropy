pub fn emit_metric() {
    metrics::counter!("orders.created");
}
