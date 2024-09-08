
use esp_idf_svc::{http::client::{Configuration, EspHttpConnection}, io::Write};
use embedded_svc::http::client::Client;

pub struct Pipe;

impl qdb::rest::Pipe for Pipe {
    fn get(&self, url: &str) -> qdb::Result<String> {
        let connection = EspHttpConnection::new(&Configuration {
            ..Default::default()
        })?;

        let mut client = Client::wrap(connection);

        let mut response = client.get(url)?.submit()?;
        let status = response.status();
        match status {
            200..=299 => {
                let mut body = String::new();
                let mut buf = [0_u8; 256];
                
                loop {
                    let bytes_read = response.read(&mut buf)?;
                    if bytes_read == 0 {
                        break;
                    }

                    body.push_str(std::str::from_utf8(&buf[..bytes_read])?);
                }

                Ok(body)
            },
            _ => Err(qdb::Error::from_client(format!("Bad status code: {}", status).as_str())),
        }
    }

    fn post(&self, url: &str, payload: &str) -> qdb::Result<String> {
        let connection = EspHttpConnection::new(&Configuration {
            ..Default::default()
        })?;

        let mut client = Client::wrap(connection);

        let mut request = client.post(url, &[])?;
        request.write_all(payload.as_bytes())?;

        let mut response = request.submit()?;
        let status = response.status();
        match status {
            200..=299 => {
                let mut body = String::new();
                let mut buf = [0_u8; 256];
                
                loop {
                    let bytes_read = response.read(&mut buf)?;
                    if bytes_read == 0 {
                        break;
                    }

                    body.push_str(std::str::from_utf8(&buf[..bytes_read])?);
                }

                Ok(body)
            },
            _ => Err(qdb::Error::from_client(format!("Bad status code: {}", status).as_str())),
        }
    }
}