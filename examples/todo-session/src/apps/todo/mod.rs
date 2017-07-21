pub mod boot;
mod controllers;

#[derive(Default, Serialize, Deserialize)]
struct Session {
    todo_list: Vec<String>,
}
