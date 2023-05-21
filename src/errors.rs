use wgpu::{CreateSurfaceError, RequestDeviceError};

#[derive(Debug)]
pub struct RenderBuddyError {
    pub message: String,
}

impl RenderBuddyError {
    pub fn new(message: impl ToString) -> Self {
        RenderBuddyError {
            message: message.to_string(),
        }
    }
}

impl From<CreateSurfaceError> for RenderBuddyError {
    fn from(e: CreateSurfaceError) -> RenderBuddyError {
        RenderBuddyError {
            message: format!("wgpu::CreateSurfaceError {:?}", &e.to_string()),
        }
    }
}

impl From<RequestDeviceError> for RenderBuddyError {
    fn from(e: RequestDeviceError) -> RenderBuddyError {
        RenderBuddyError {
            message: format!("wgpu::RequestDeviceError {:?}", &e.to_string()),
        }
    }
}
