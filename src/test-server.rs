use std::result::Result;
use tonic::{transport::Server, Request, Response, Status};

use admin::admin_service_server::{AdminService, AdminServiceServer};
use admin::{RegistryRequest, RegistryResponse, ApplicationRequest, ApplicationResponse, UnitStatus, Empty, ApplicationList,};

//update protocol first
//use givc_common::pb::admin::admin_service_server::{AdminService, AdminServiceServer};
//use givc_common::pb::admin::{RegistryRequest, RegistryResponse, ApplicationRequest, ApplicationResponse, UnitStatus, Empty, ApplicationList,};

pub mod admin {
    tonic::include_proto!("admin");
}

#[derive(Debug, Default)]
pub struct MyAdminService {}

#[tonic::async_trait]
impl AdminService for MyAdminService {
    async fn register_service(
        &self,
        request: Request<RegistryRequest>,
    ) -> Result<Response<RegistryResponse>, Status> {
        let resp = RegistryResponse::default();
        Ok(Response::new(resp))
    }

    async fn start_application(
        &self,
        request: Request<ApplicationRequest>,
    ) -> Result<Response<ApplicationResponse>, Status> {
        let resp = ApplicationResponse::default();
        Ok(Response::new(resp))
    }

    async fn pause_application(
        &self,
        request: Request<ApplicationRequest>,
    ) -> Result<Response<ApplicationResponse>, Status> {
        let resp = ApplicationResponse::default();
        Ok(Response::new(resp))
    }

    async fn resume_application(
        &self,
        request: Request<ApplicationRequest>,
    ) -> Result<Response<ApplicationResponse>, Status> {
        let resp = ApplicationResponse::default();
        Ok(Response::new(resp))
    }

    async fn stop_application(
        &self,
        request: Request<ApplicationRequest>,
    ) -> Result<Response<ApplicationResponse>, Status> {
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

        Ok(Response::new(reply)) // Send back
    }

    async fn poweroff(
        &self,
        request: Request<Empty>,
    ) -> Result<Response<Empty>, Status> {
        let resp = Empty::default();
        Ok(Response::new(resp))
    }

    async fn reboot(
        &self,
        request: Request<Empty>,
    ) -> Result<Response<Empty>, Status> {
        let resp = Empty::default();
        Ok(Response::new(resp))
    }

    async fn list_applications(
        &self,
        request: Request<Empty>,
    ) -> Result<Response<ApplicationList>, Status,> {
        let apps = vec![
            UnitStatus {
                name: "VM1".to_string(),
                description: "This is file.pdf!".to_string(),
                load_state: "Loaded".to_string(),
                active_state: "active".to_string(),
                path: "/test/path".to_string(),
            },
            UnitStatus {
                name: "VM2".to_string(),
                description: "Google Chrome".to_string(),
                load_state: "Loaded".to_string(),
                active_state: "active".to_string(),
                path: "/test/path2".to_string(),
            },
        ];

        Ok(Response::new(ApplicationList {applications: apps,}))
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