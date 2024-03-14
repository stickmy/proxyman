use tauri::{LogicalSize, Size, Window};

pub fn initial_window(win: Window) -> () {

    win.center().unwrap();

    win
        .set_size(Size::Logical(LogicalSize {
            width: 1200.0,
            height: 800.0,
        }))
        .unwrap();
}