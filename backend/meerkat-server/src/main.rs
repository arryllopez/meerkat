mod types;
mod messages;
fn main() {
    tracing_subscriber::fmt() 
        .json() 
        .init(); 


}
