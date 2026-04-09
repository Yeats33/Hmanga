use dioxus::prelude::*;

#[component]
pub fn Sidebar() -> Element {
    rsx! {
        nav {
            style: "width: 240px; background: #1a1a2e; color: white; padding: 24px 16px; display: flex; flex-direction: column; gap: 8px;",

            div {
                style: "font-size: 28px; font-weight: bold; margin-bottom: 32px; text-align: center;",
                "Hmanga"
            }

            div {
                style: "padding: 12px 16px; background: rgba(255,255,255,0.1); border-radius: 8px; font-weight: 500;",
                "首页"
            }
            
            div {
                style: "padding: 12px 16px; border-radius: 8px; cursor: pointer; transition: background 0.2s;",
                "搜索"
            }
            
            div {
                style: "padding: 12px 16px; border-radius: 8px; cursor: pointer; transition: background 0.2s;",
                "下载"
            }
            
            div {
                style: "padding: 12px 16px; border-radius: 8px; cursor: pointer; transition: background 0.2s;",
                "设置"
            }
        }
    }
}
