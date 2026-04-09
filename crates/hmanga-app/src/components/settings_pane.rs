use dioxus::prelude::*;

#[component]
pub fn SettingsPane(donation_unlocked: Signal<bool>) -> Element {
    let unlocked = donation_unlocked.read().clone();

    rsx! {
        div {
            style: "flex: 1; padding: 24px; overflow-y: auto;",

            h1 {
                style: "font-size: 24px; font-weight: 600; margin-bottom: 24px; color: #333;",
                "设置"
            }

            div {
                style: "padding: 20px; background: linear-gradient(135deg, #667eea 0%, #764ba2 100%); border-radius: 12px; color: white;",

                h3 {
                    style: "margin: 0 0 8px 0; font-size: 18px;",
                    "插件解锁"
                }

                p {
                    style: "margin: 8px 0; opacity: 0.9; font-size: 14px;",
                    "感谢支持！解锁所有官方插件。"
                }

                if !unlocked {
                    button {
                        style: "margin-top: 16px; padding: 10px 20px; background: white; color: #667eea; border: none; border-radius: 6px; cursor: pointer; font-weight: 600; font-size: 14px;",
                        onclick: move |_| donation_unlocked.set(true),
                        "我已捐献，解锁插件"
                    }
                } else {
                    div {
                        style: "margin-top: 16px; padding: 8px 12px; background: rgba(255,255,255,0.2); border-radius: 6px; display: inline-block;",
                        span { "✓ 插件已解锁" }
                    }
                }
            }

            div {
                style: "margin-top: 24px; padding: 20px; background: #f8f9fa; border-radius: 12px;",

                h3 {
                    style: "margin: 0 0 16px 0; font-size: 18px; color: #333;",
                    "官方插件"
                }

                div {
                    style: "display: flex; flex-direction: column; gap: 12px;",

                    div {
                        style: "display: flex; justify-content: space-between; align-items: center; padding: 16px; background: white; border-radius: 8px; box-shadow: 0 2px 4px rgba(0,0,0,0.05);",

                        div {
                            style: "display: flex; flex-direction: column;",
                            span { style: "font-weight: 600; font-size: 16px;", "J-Manga" }
                            span { style: "font-size: 13px; color: #666;", "禁漫天堂官方插件" }
                        }

                        div {
                            style: "padding: 6px 12px; border-radius: 20px; font-size: 13px; font-weight: 600;",
                            if unlocked {
                                span { style: "color: #4CAF50; background: rgba(76,175,80,0.1);", "已解锁" }
                            } else {
                                span { style: "color: #ff9800; background: rgba(255,152,0,0.1);", "未解锁" }
                            }
                        }
                    }
                }
            }
        }
    }
}
