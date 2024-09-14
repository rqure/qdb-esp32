use std::sync::mpsc::Receiver;

use esp_idf_svc::hal::{gpio::{Input, InputPin, OutputPin, PinDriver, Pull}, peripheral::Peripheral};

pub struct Receivers {
    pub db_connection_status: Option<Receiver<bool>>,
}

pub struct Worker<'a, T: InputPin> {
    pub receivers: Receivers,

    is_db_connected: bool,
    write_complete: bool,
    pin: PinDriver<'a, T, Input>,
}

impl<'a, T: InputPin + OutputPin> Worker<'a, T> {
    pub fn new(pin: impl Peripheral<P = T> + 'a) -> Self {
        Self {
            receivers: Receivers {
                db_connection_status: None,
            },
            is_db_connected: false,
            write_complete: false,
            pin: PinDriver::input(pin).expect("Failed to initialize pin driver"),
        }
    }
}

impl<'a, T: InputPin + OutputPin> qdb::framework::workers::common::WorkerTrait for Worker<'a, T> {
    fn intialize(&mut self, ctx: qdb::framework::application::Context) -> qdb::Result<()> {
        let c = format!("{}::{}", std::any::type_name::<Self>(), "initialize");

        ctx.logger().info(format!("[{}] Initializing Remote worker", c).as_str());

        self.pin.set_pull(Pull::Down)?;

        Ok(())
    }

    fn do_work(&mut self, ctx: qdb::framework::application::Context) -> qdb::Result<()> {
        let c = format!("{}::{}", std::any::type_name::<Self>(), "do_work");

        if !self.is_db_connected {
            return Ok(());
        }

        if !self.pin.is_high() {
            if !self.write_complete {
                ctx.logger().info(format!("[{}] Remote button pressed", c).as_str());

                let doors = ctx.database().find("GarageDoor", &vec![], |_| true)?;

                doors.iter().for_each(|door| {
                    ctx.database().write(&vec![{
                        door.field("OpenTrigger").set_i64_value(0).clone()
                    }]).ok();
                });

                self.write_complete = true;
            }
        } else {
            self.write_complete = false;
        }

        Ok(())
    }

    fn deinitialize(&mut self, ctx: qdb::framework::application::Context) -> qdb::Result<()> {
        let c = format!("{}::{}", std::any::type_name::<Self>(), "deinitialize");

        ctx.logger().info(format!("[{}] Deinitializing Remote worker", c).as_str());

        Ok(())
    }

    fn process_events(&mut self) -> qdb::Result<()> {
        if let Some(db_connection_status) = &self.receivers.db_connection_status {
            if let Ok(is_connected) = db_connection_status.try_recv() {
                self.is_db_connected = is_connected;
            }
        }

        Ok(())
    }
}