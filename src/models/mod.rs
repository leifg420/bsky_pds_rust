use serde::{Deserialize, Serialize};
use serde;


#[derive(Debug, Serialize, Deserialize)]
pub struct User {
    pub id: u32,
    pub username: String,
    pub email: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Post {
    pub id: u32,
    pub user_id: u32,
    pub content: String,
}
