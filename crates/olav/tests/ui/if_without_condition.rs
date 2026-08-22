use olav::xml;

fn main() {
    let _ = xml! {
        root {
            @if { "x" }
        }
    };
}
