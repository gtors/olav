use olav::xml;

fn main() {
    let _ = xml! { root([bad name]="v") { "x" } };
}
