use std::sync::{Arc, Mutex};
use std::error::Error;
use std::thread;
use std::sync::mpsc::{self, Sender, Receiver};
use tonic::{transport::Server, transport::Channel, Request, Response, Status};

use admin::admin_service_client::{AdminServiceClient};
use admin::{RegistryRequest, RegistryResponse, ApplicationRequest, ApplicationResponse, UnitStatus, Empty};

//to communicate with admin service and get data
pub mod admin {
    tonic::include_proto!("admin");
}

pub mod client {
    use super::*;

    //#[derive(Debug, Clone)]
    pub struct ClientService {
        admin_client: AdminServiceClient<Channel>,
    }
    
    impl ClientService {
        pub async fn new(endpoint: String) -> Result<Self, Box<dyn Error>> {
            let client = AdminServiceClient::connect(endpoint).await?;
            Ok(Self { admin_client: client })
        }
    
        pub async fn app_request(&mut self) -> Result<(), Box<dyn Error>> {
            let request = tonic::Request::new(ApplicationRequest {
                app_name: "Test".into(),
            });
    
            //let mut admin_client = self.admin_client.clone();
            let response = self.admin_client.application_status(request).await?;
    
            println!("RESPONSE={:?}", response);
            Ok(())
        }
    }

    pub enum ClientServiceRequest {
        AppRequest,
    }

    pub enum ClientServiceResponse {
        AppResponse(Result<String, Box<dyn Error>>),
    }
    
    pub fn client_service_thread(endpoint: String, request_receiver: Receiver<ClientServiceRequest>/*, response_sender: Sender<ClientServiceResponse>*/) {
        tokio::runtime::Runtime::new().unwrap().block_on(async move {
            let mut client_service = client::ClientService::new(endpoint).await.unwrap();
    
            while let Ok(request) = request_receiver.recv() {
                match request {
                    ClientServiceRequest::AppRequest => {
                        let response = client_service.app_request().await;
                        println!("RESPONSE={:?}", response);
                        //response_sender.send(ClientServiceResponse::AppResponse(response)).unwrap();
                    }
                }
            }
        });
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut admin_client = client::ClientService::new("http://[::1]:50051".to_string()).await?;
    admin_client.app_request().await?;
    Ok(())
}