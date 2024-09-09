use std::sync::mpsc::Receiver;

use esp_idf_svc::hal::{gpio::{Input, InputPin, OutputPin, PinDriver, Pull}, peripheral::Peripheral};
use qdb::{DatabaseField, DatabaseValue, RawField, RawValue};

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

impl<'a, T: InputPin + OutputPin> qdb::WorkerTrait for Worker<'a, T> {
    fn intialize(&mut self, ctx: qdb::ApplicationContext) -> qdb::Result<()> {
        ctx.logger().info("[Remote::initialize] Initializing Remote worker");

        self.pin.set_pull(Pull::Down)?;

        Ok(())
    }

    fn do_work(&mut self, ctx: qdb::ApplicationContext) -> qdb::Result<()> {
        if !self.is_db_connected {
            return Ok(());
        }

        if !self.pin.is_high() {
            if !self.write_complete {
                ctx.logger().info("[Remote::do_work] Remote button pressed");

                let doors = ctx.database().find("AudioController", &vec![], |_| true)?;

                doors.iter().for_each(|door| {
                    let mut field = RawField::new(door.entity_id.clone(), "TextToSpeech");
                    field.value = DatabaseValue::new(RawValue::String("Button pressed".to_string()));
                    ctx.database().write(&vec![{
                        DatabaseField::new(field)
                    }]).ok();
                });

                self.write_complete = true;
            }
        } else {
            self.write_complete = false;
        }

        Ok(())
    }

    fn deinitialize(&mut self, ctx: qdb::ApplicationContext) -> qdb::Result<()> {
        ctx.logger().info("[Remote::deinitialize] Deinitializing Remote worker");

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