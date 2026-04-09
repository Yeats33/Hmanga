use dioxus::prelude::*;

#[component]
pub fn DownloadsPane() -> Element {
    rsx! {
        div {
            style: "flex: 1; padding: 24px; overflow-y: auto;",

            h2 {
                style: "font-size: 20px; font-weight: 600; margin-bottom: 16px; color: #333;",
                "下载管理"
            }

            div {
                style: "padding: 20px; background: #f8f9fa; border-radius: 8px; text-align: center;",
                p { style: "color: #666;", "暂无下载任务" }
            }
        }
    }
}
