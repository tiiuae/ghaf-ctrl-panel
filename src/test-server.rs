use tonic::{transport::Server, Request, Response, Status};

use admin::admin_service_server::{AdminService, AdminServiceServer};
use admin::{RegistryRequest, RegistryResponse, ApplicationRequest, ApplicationResponse, UnitStatus, Empty};

//to communicate with admin service and get data
pub mod admin {
    tonic::include_proto!("admin");
}

#[derive(Debug, Default)]
pub struct MyAdminService {}

#[tonic::async_trait]
impl AdminService for MyAdminService {
    async fn register_service(
        &self,
        request: tonic::Request<RegistryRequest>,
    ) -> std::result::Result<tonic::Response<RegistryResponse>, tonic::Status> {
        let resp = RegistryResponse::default();
        Ok(Response::new(resp))
    }

    async fn start_application(
        &self,
        request: tonic::Request<ApplicationRequest>,
    ) -> std::result::Result<tonic::Response<ApplicationResponse>, tonic::Status> {
        let resp = ApplicationResponse::default();
        Ok(Response::new(resp))
    }

    async fn pause_application(
        &self,
        request: tonic::Request<ApplicationRequest>,
    ) -> std::result::Result<tonic::Response<ApplicationResponse>, tonic::Status> {
        let resp = ApplicationResponse::default();
        Ok(Response::new(resp))
    }

    async fn resume_application(
        &self,
        request: tonic::Request<ApplicationRequest>,
    ) -> std::result::Result<tonic::Response<ApplicationResponse>, tonic::Status> {
        let resp = ApplicationResponse::default();
        Ok(Response::new(resp))
    }

    async fn stop_application(
        &self,
        request: tonic::Request<ApplicationRequest>,
    ) -> std::result::Result<tonic::Response<ApplicationResponse>, tonic::Status> {
        let resp = ApplicationResponse::default();
        Ok(Response::new(resp))
    }

    async fn application_status(
        &self,
        request: Request<ApplicationRequest>,
    ) -> Result<Response<UnitStatus>, Status> {
        println!("Got a request: {:?}", request);

        let reply = UnitStatus {
            name: request.into_inner().app_name, // We must use .into_inner() as the fields of gRPC requests and responses are private
            description: String::from("Test"),
            load_state: String::from("unknown"),
            active_state: String::from("unknown"),
            path: String::from("test"),
        };

        Ok(Response::new(reply)) // Send back our formatted greeting
    }

    async fn poweroff(
        &self,
        request: tonic::Request<Empty>,
    ) -> std::result::Result<tonic::Response<Empty>, tonic::Status> {
        let resp = Empty::default();
        Ok(Response::new(resp))
    }

    async fn reboot(
        &self,
        request: tonic::Request<Empty>,
    ) -> std::result::Result<tonic::Response<Empty>, tonic::Status> {
        let resp = Empty::default();
        Ok(Response::new(resp))
    }

}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = "[::1]:50051".parse()?;
    let admin = MyAdminService::default();

    Server::builder()
        .add_service(AdminServiceServer::new(admin))
        .serve(addr)
        .await?;

    Ok(())
}