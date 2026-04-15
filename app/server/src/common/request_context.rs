#[derive(Clone, Debug)]
pub struct RequestContext {
    request_id: String,
}

impl RequestContext {
    pub fn new(request_id: impl Into<String>) -> Self {
        Self {
            request_id: request_id.into(),
        }
    }

    pub fn request_id(&self) -> &str {
        &self.request_id
    }
}
