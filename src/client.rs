use std::sync::{Arc, Mutex};
use std::error::Error;
use std::thread;
use std::net::SocketAddr;
use std::sync::mpsc::{self, Sender, Receiver};
use tokio::time::{sleep, Duration};

use givc_client::{self, AdminClient, client::QueryResult, client::Event};
use givc_client::endpoint::{EndpointConfig, TlsConfig};
use givc_common::pb::TransportConfig;
use givc_common::types::*;

pub mod client {
    use super::*;
    
    pub struct ClientService {
        admin_client: AdminClient,
    }
    
    impl ClientService {
        pub async fn connect(addr: String, port: u16) -> Result<Self, Box<dyn Error>> {
            let admin_cfg = EndpointConfig {
                transport: EndpointEntry {
                    address: addr,
                    port: port,
                    protocol: "bogus".into(),
                },
                tls: None, // No TLS in cli at the moment
            };

            let client = AdminClient::new(admin_cfg);

            Ok(Self {admin_client: client})
        }

        /*
        pub async fn try_new(endpoint: String) -> Result<Self, Box<dyn Error>> {
            let mut client_service = loop {
                match client::ClientService::new(endpoint.clone()).await {
                    Ok(service) => {
                        println!("Client service created, connection established!");
                        break service;
                    }
                    Err(e) => {
                        eprintln!("Error creating client service: {}. Retrying in 5 seconds...", e);
                        sleep(Duration::from_secs(5)).await;
                    }
                }
            };
            Ok(client_service)
        }

        pub async fn reconnect(&mut self, endpoint: String) -> Result<(), Box<dyn Error>> {
            self.admin_client = AdminClient::connect(endpoint).await?;
            Ok(())
        }
        
        pub async fn application_list_request(&mut self) -> Result<Response<UnitStatus>, Status> {
            let request = Request::new(app_request);
    
            let response = self.admin_client.query_list(request).await?;
    
            println!("RESPONSE={:?}", response);
            Ok(response)
        }
        */
    }

    pub enum ClientServiceRequest {
        Register(),
        AppList(),
        AppStatus(String),
        StartApp(String),
        PauseApp(String),
        StopApp(String),
    }
    
    pub fn client_service_thread<F>(endpoint: String, request_receiver: Receiver<ClientServiceRequest>, response_sender: Sender<Event>, callback: F) 
    where F: Fn(Event),
    {
        tokio::runtime::Runtime::new().unwrap().block_on(async move {
            let mut client_service = loop {
                match client::ClientService::connect(String::from("http://[::1]"), 50051).await {
                    Ok(service) => {
                        println!("Client service created, connection established!");//send connected event!
                        break service;
                    }
                    Err(e) => {
                        eprintln!("Error creating client service: {}. Retrying in 5 seconds...", e);
                        sleep(Duration::from_secs(5)).await;
                    }
                }
            };
    
            //TODO: reconnect when send error occurs!
            while let Ok(request) = request_receiver.recv() {
                match request {
                    ClientServiceRequest::Register() => {
                        /*let response = client_service.admin_client.register_service(Request::new(registry_request)).await;
                        println!("RESPONSE={:?}", response);
                        match response {
                            Ok(value) => response_sender.send(ClientServiceResponse::Register(value.into_inner())).expect("Send error"),
                            Err(e) => println!("Registering error!"),
                        }*/
                    }
                    ClientServiceRequest::AppList() => {
                        let response = client_service.admin_client.query_list().await;
                        println!("RESPONSE={:?}", response);
                        match response {
                            Ok(value) => response_sender.send(Event::ListUpdate(value)).expect("Send error"),//callback(ClientServiceResponse::AppList(value.into_inner())),
                            Err(e) => println!("App list error!"),
                        }
                    }
                    ClientServiceRequest::AppStatus(app_name) => {
                        /*let response = client_service.admin_client.application_status(Request::new(app_request)).await;
                        println!("RESPONSE={:?}", response);
                        match response {
                            Ok(value) => response_sender.send(ClientServiceResponse::Status(value.into_inner())).expect("Send error"),
                            Err(e) => println!("App status error!"),
                        }*/
                    }
                    ClientServiceRequest::StartApp(app_name) => {
                        /*let response = client_service.admin_client.start_application(Request::new(app_request)).await;
                        println!("RESPONSE={:?}", response);
                        match response {
                            Ok(value) => response_sender.send(ClientServiceResponse::AppStatus(value.into_inner())).expect("Send error"),
                            Err(e) => println!("start app error!"),
                        }*/
                    }
                    ClientServiceRequest::PauseApp(app_name) => {
                        /*let response = client_service.admin_client.pause_application(Request::new(app_request)).await;
                        println!("RESPONSE={:?}", response);
                        match response {
                            Ok(value) => response_sender.send(ClientServiceResponse::AppStatus(value.into_inner())).expect("Send error"),
                            Err(e) => println!("pause error!"),
                        }*/
                    }
                    ClientServiceRequest::StopApp(app_name) => {
                        /*let response = client_service.admin_client.stop_application(Request::new(app_request)).await;
                        println!("RESPONSE={:?}", response);
                        match response {
                            Ok(value) => response_sender.send(ClientServiceResponse::AppStatus(value.into_inner())).expect("Send error"),
                            Err(e) => println!("stop error!"),
                        }*/
                    }
                }
            }
        });
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    //let mut admin_client = client::ClientService::new("http://[::1]:50051".to_string()).await?;
    //admin_client.application_status_request(ApplicationRequest {app_name: "Test".into(),}).await?;
    Ok(())
}
