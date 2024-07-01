use tonic::{transport::Server, Request, Response, Status};

use admin::admin_service_client::{AdminServiceClient};
use admin::{RegistryRequest, RegistryResponse, ApplicationRequest, ApplicationResponse, UnitStatus, Empty};

//to communicate with admin service and get data
pub mod admin {
    tonic::include_proto!("admin");
}

async fn app_request() -> Result<(), Box<dyn std::error::Error>> {
    let mut client = AdminServiceClient::connect("http://[::1]:50051").await?;

    let request = tonic::Request::new(ApplicationRequest {
        app_name: "Test".into(),
    });

    let response = client.application_status(request).await?;

    println!("RESPONSE={:?}", response);
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    app_request().await?;
    Ok(())
}