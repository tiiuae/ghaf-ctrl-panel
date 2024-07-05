use std::sync::{Arc, Mutex};
use std::error::Error;
use std::thread;
use std::sync::mpsc::{self, Sender, Receiver};
use tokio::time::{sleep, Duration};
use tonic::{transport::Server, transport::Channel, Request, Response, Status};

use admin::admin_service_client::{AdminServiceClient};
use admin::{RegistryRequest, RegistryResponse, ApplicationRequest, ApplicationResponse, UnitStatus, ApplicationList, Empty};

pub mod admin {
    tonic::include_proto!("admin");
}

pub mod client {
    use super::*;

    pub struct ClientService {
        admin_client: AdminServiceClient<Channel>,
        //+Arc to VMGobject's ListStore?
    }
    
    impl ClientService {
        pub async fn new(endpoint: String) -> Result<Self, Box<dyn Error>> {
            let client = AdminServiceClient::connect(endpoint).await?;
            Ok(Self { admin_client: client })
        }

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
            self.admin_client = AdminServiceClient::connect(endpoint).await?;
            Ok(())
        }
    
        pub async fn application_status_request(&mut self, app_request: ApplicationRequest) -> Result<Response<UnitStatus>, Status> {
            let request = Request::new(app_request);
    
            let response = self.admin_client.application_status(request).await?;
    
            println!("RESPONSE={:?}", response);
            Ok(response)
        }
    }

    pub enum ClientServiceRequest {
        Register(RegistryRequest),
        AppList(),
        AppStatus(ApplicationRequest),
        StartApp(ApplicationRequest),
        PauseApp(ApplicationRequest),
        StopApp(ApplicationRequest),
    }

    pub enum ClientServiceResponse {
        Register(RegistryResponse),
        AppList(ApplicationList),
        AppStatus(ApplicationResponse),
        Status(UnitStatus),
        Empty(),
    }
    
    pub fn client_service_thread(endpoint: String, request_receiver: Receiver<ClientServiceRequest>, response_sender: Sender<ClientServiceResponse>) {
        tokio::runtime::Runtime::new().unwrap().block_on(async move {
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
    
            //reconnect when send error occurs!
            while let Ok(request) = request_receiver.recv() {
                match request {
                    ClientServiceRequest::Register(registry_request) => {
                        let response = client_service.admin_client.register_service(Request::new(registry_request)).await;
                        println!("RESPONSE={:?}", response);
                        match response {
                            Ok(value) => response_sender.send(ClientServiceResponse::Register(value.into_inner())).expect("Send error"),
                            Err(e) => println!("Registering error!"),
                        }
                    }
                    ClientServiceRequest::AppList() => {
                        let response = client_service.admin_client.list_applications(Request::new(Empty{})).await;
                        println!("RESPONSE={:?}", response);
                        match response {
                            Ok(value) => response_sender.send(ClientServiceResponse::AppList(value.into_inner())).expect("Send error"),
                            Err(e) => println!("App list error!"),
                        }
                        //fill ListStore
                    }
                    ClientServiceRequest::AppStatus(app_request) => {
                        let response = client_service.admin_client.application_status(Request::new(app_request)).await;
                        println!("RESPONSE={:?}", response);
                        match response {
                            Ok(value) => response_sender.send(ClientServiceResponse::Status(value.into_inner())).expect("Send error"),
                            Err(e) => println!("App status error!"),
                        }
                    }
                    ClientServiceRequest::StartApp(app_request) => {
                        let response = client_service.admin_client.start_application(Request::new(app_request)).await;
                        println!("RESPONSE={:?}", response);
                        match response {
                            Ok(value) => response_sender.send(ClientServiceResponse::AppStatus(value.into_inner())).expect("Send error"),
                            Err(e) => println!("start app error!"),
                        }
                    }
                    ClientServiceRequest::PauseApp(app_request) => {
                        let response = client_service.admin_client.pause_application(Request::new(app_request)).await;
                        println!("RESPONSE={:?}", response);
                        match response {
                            Ok(value) => response_sender.send(ClientServiceResponse::AppStatus(value.into_inner())).expect("Send error"),
                            Err(e) => println!("pause error!"),
                        }
                    }
                    ClientServiceRequest::StopApp(app_request) => {
                        let response = client_service.admin_client.stop_application(Request::new(app_request)).await;
                        println!("RESPONSE={:?}", response);
                        match response {
                            Ok(value) => response_sender.send(ClientServiceResponse::AppStatus(value.into_inner())).expect("Send error"),
                            Err(e) => println!("stop error!"),
                        }
                    }
                }
            }
        });
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut admin_client = client::ClientService::new("http://[::1]:50051".to_string()).await?;
    admin_client.application_status_request(ApplicationRequest {app_name: "Test".into(),}).await?;
    Ok(())
}