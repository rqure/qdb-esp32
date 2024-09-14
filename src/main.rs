mod workers;
mod pipe;
mod auth;

use esp_idf_svc::{eventloop::EspSystemEventLoop, hal::{gpio::IOPin, prelude::Peripherals}};

use qdb::framework::application::{Application, ApplicationTrait, Context};
use qdb::framework::database::Database;
use qdb::framework::client::Client;
use qdb::framework::logger::Logger;

fn main() {
    // It is necessary to call this function once. Otherwise some patches to the runtime
    // implemented by esp-idf-sys might not link properly. See https://github.com/esp-rs/esp-idf-template/issues/71
    esp_idf_svc::sys::link_patches();

    // Bind the log crate to the ESP Logging facilities
    esp_idf_svc::log::EspLogger::initialize_default();

    let peripherals = Peripherals::take().expect("Failed to take peripherals");
    let sysloop = EspSystemEventLoop::take().expect("Failed to take system event loop");

    let ctx = Context::new(
        Database::new(
            Client::new(
                qdb::clients::rest::Client::new("http://qserver.local", Box::new(pipe::Pipe))
            ),
        ),
        Logger::new(
            qdb::loggers::console::Console::new(qdb::loggers::common::LogLevel::Trace))
    );

    let loop_interval_ms = 100;
    let mut app = Application::new(ctx, loop_interval_ms);

    let mut db_worker = Box::new(qdb::framework::workers::database::Worker::new());
    let mut wifi_worker = Box::new(workers::wifi::Worker::new(auth::SSID, auth::PASSWORD, peripherals.modem, sysloop));
    let mut remote_worker = Box::new(workers::remote::Worker::new(peripherals.pins.gpio0.downgrade()));

    db_worker.receivers.network_connection_status = Some(wifi_worker.emitters.connection_status.new_receiver());
    remote_worker.receivers.db_connection_status = Some(db_worker.emitters.connection_status.new_receiver());
    
    app.add_worker(wifi_worker);
    app.add_worker(db_worker);
    app.add_worker(remote_worker);

    app.execute();
}
