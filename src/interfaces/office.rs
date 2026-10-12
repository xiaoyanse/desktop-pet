use crate::domain::office::Book;

pub trait OfficeStore: Send + Sync {
    fn load(&self) -> Result<Book, String>;
    fn save(&self, book: &Book, previous: &Book) -> Result<(), String>;
    fn location(&self) -> String;
}
pub trait LocalTime: Send + Sync {
    fn format(&self, unix_seconds: u64) -> String;
    fn parse(&self, text: &str) -> Result<u64, String>;
}
