use log::trace;

#[tokio::main]
async fn main() {
    trace!("main() called");
    liro::run().await
}
