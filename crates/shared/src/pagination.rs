use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageRequest {
    pub page: u32,
    pub per_page: u32,
}

impl Default for PageRequest {
    fn default() -> Self {
        Self { page: 1, per_page: 20 }
    }
}

impl PageRequest {
    pub fn offset(&self) -> i64 {
        ((self.page.saturating_sub(1)) as i64) * self.per_page as i64
    }
    pub fn limit(&self) -> i64 {
        self.per_page as i64
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageResponse<T> {
    pub items: Vec<T>,
    pub total: i64,
    pub page: u32,
    pub per_page: u32,
}

impl<T> PageResponse<T> {
    pub fn new(items: Vec<T>, total: i64, req: &PageRequest) -> Self {
        Self {
            items,
            total,
            page: req.page,
            per_page: req.per_page,
        }
    }
}
