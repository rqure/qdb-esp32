mod workers;
mod pipe;

use esp_idf_svc::{eventloop::EspSystemEventLoop, hal::{gpio::{IOPin, PinDriver}, prelude::Peripherals}};
use qdb::{ApplicationTrait, ConsoleLogger};

fn main() {
    // It is necessary to call this function once. Otherwise some patches to the runtime
    // implemented by esp-idf-sys might not link properly. See https://github.com/esp-rs/esp-idf-template/issues/71
    esp_idf_svc::sys::link_patches();

    // Bind the log crate to the ESP Logging facilities
    esp_idf_svc::log::EspLogger::initialize_default();

    let peripherals = Peripherals::take().expect("Failed to take peripherals");
    let sysloop = EspSystemEventLoop::take().expect("Failed to take system event loop");

    let ctx = qdb::ApplicationContext::new(
        qdb::Database::new(
            qdb::rest::Client::new("http://qserver.local", Box::new(pipe::Pipe)),
        ),
        qdb::Logger::new(ConsoleLogger::new(qdb::LogLevel::Debug))
    );

    let loop_interval_ms = 500;
    let mut app = qdb::Application::new(ctx, loop_interval_ms);

    let mut db_worker = Box::new(qdb::DatabaseWorker::new());
    let mut wifi_worker = Box::new(workers::wifi::Worker::new("SSID", "PASSWORD", peripherals.modem, sysloop));
    let mut remote_worker = Box::new(workers::remote::Worker::new(peripherals.pins.gpio0.downgrade()));

    db_worker.network_connection_events = Some(wifi_worker.emitters.connection_status.new_receiver());
    remote_worker.receivers.db_connection_status = Some(db_worker.emitters.connection_status.new_receiver());
    
    app.add_worker(wifi_worker);
    app.add_worker(db_worker);
    app.add_worker(remote_worker);

    app.execute();
}
