use iced_core::Clipboard;
pub struct LayerShellClipboard;

// TODO: clipboard
impl Clipboard for LayerShellClipboard {
    fn read(&self, _kind: iced_core::clipboard::Kind) -> Option<String> {
        None
    }
    fn write(&mut self, _kind: iced_core::clipboard::Kind, _contents: String) {}
}
