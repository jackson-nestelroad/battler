use std::sync::Once;

static INIT: Once = Once::new();

pub fn setup_test_environment() {
    INIT.call_once(|| {
        tracing_subscriber::fmt()
            .with_max_level(tracing_core::Level::DEBUG)
            .with_file(true)
            .with_line_number(true)
            .with_thread_ids(true)
            .init();
    });
}
