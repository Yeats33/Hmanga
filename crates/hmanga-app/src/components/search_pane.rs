use dioxus::prelude::*;

#[component]
pub fn SearchPane() -> Element {
    rsx! {
        div {
            style: "flex: 1; padding: 24px; overflow-y: auto;",

            h2 {
                style: "font-size: 20px; font-weight: 600; margin-bottom: 16px; color: #333;",
                "搜索漫画"
            }

            div {
                style: "display: flex; gap: 12px; margin-bottom: 24px;",
                
                input {
                    style: "flex: 1; padding: 12px 16px; border: 2px solid #e0e0e0; border-radius: 8px; font-size: 14px;",
                    placeholder: "输入漫画名称或关键词..."
                }
                
                button {
                    style: "padding: 12px 24px; background: #1a1a2e; color: white; border: none; border-radius: 8px; cursor: pointer; font-weight: 500;",
                    "搜索"
                }
            }

            div {
                style: "padding: 20px; background: #f8f9fa; border-radius: 8px; text-align: center;",
                p { style: "color: #666;", "输入关键词开始搜索漫画" }
            }
        }
    }
}
