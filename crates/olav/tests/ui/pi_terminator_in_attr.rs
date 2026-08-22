use olav::xml;

fn main() {
    let _ = xml! {
        ?xml version="1?>0"
        root { "x" }
    };
}
