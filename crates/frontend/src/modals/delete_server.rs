use bridge::handle::BackendHandle;
use bridge::message::MessageToBackend;
use gpui::{prelude::*, *};
use gpui_component::{
    button::{Button, ButtonVariants}, h_flex, WindowExt
};
use crate::root;

pub fn open_delete_server(
    server_name: SharedString,
    backend_handle: BackendHandle,
    window: &mut Window,
    cx: &mut App,
) {
    window.open_dialog(cx, move |dialog, _, _| {
        dialog
            .title("Delete Server")
            .overlay_closable(false)
            .flex()
            .line_height(rems(1.2))
            .child(format!("Are you sure you want to delete '{}'?", server_name))
            .footer(h_flex()
                .gap_2()
                .w_full()
                .child(
                    div()
                        .flex_1()
                        .child(
                            Button::new("cancel_delete")
                                .info()
                                .w_full()
                                .label("Cancel")
                                .on_click(|_, window, cx| {
                                    window.close_dialog(cx);
                                })
                        )
                )
                .child(
                    div()
                        .flex_1()
                        .child(
                            Button::new("confirm_delete")
                                .info()
                                .w_full()
                                .label("Delete Server")
                                .on_click({
                                    let backend_handle = backend_handle.clone();
                                    let server_name = server_name.clone();
                                    move |_, window, cx| {
                                        backend_handle.send(MessageToBackend::DeleteServer {
                                            name: server_name.as_str().into(),
                                        });
                                        window.close_dialog(cx);
                                        root::switch_page(
                                            crate::ui::PageType::Servers,
                                            &[],
                                            window,
                                            cx,
                                        );
                                    }
                                })
                        )
                )
            )
    });
}
