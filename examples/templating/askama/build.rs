extern crate askama;

fn main() {
    // Instruct askama to rebuild the templates if they've changed.
    // This can be removed if the templates are not expected to change; remember to rebuild the crate if they do, however.
    askama::rerun_if_templates_changed();
}