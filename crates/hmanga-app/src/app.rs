use dioxus::prelude::*;
use std::rc::Rc;

use crate::service::{to_browser_src, AppServices, LocalChapterEntry};
use crate::state::{
    BrowseTab, DownloadPanelTab, DownloadRow, DownloadRowState, ReaderState, SiteTab, UiState,
    WorkspaceTab,
};

#[component]
pub fn App() -> Element {
    let services = use_signal(AppServices::new);
    let mut ui = use_signal(UiState::default);
    let mut bootstrapped = use_signal(|| false);

    if !*bootstrapped.read() {
        bootstrapped.set(true);
        let services = services.read().clone();
        let mut ui_handle = ui;
        spawn(async move {
            let library = services.read_library().unwrap_or_default();
            let config = services.config();
            let saved_username = config.jm_username.clone();
            let saved_password = config.jm_password.clone();
            ui_handle.with_mut(|state| {
                state.library = library;
                state.jm_username = saved_username.clone();
                state.jm_password = saved_password.clone();
                state.settings_config = config.clone();
                state.status = format!(
                    "配置：{} ｜ 下载目录：{}",
                    services.config_path().to_string_lossy(),
                    config.download_dir.to_string_lossy()
                );
            });

            if !saved_username.is_empty() && !saved_password.is_empty() {
                ui_handle.with_mut(|state| {
                    state.status = "检测到已保存的 JM 账号，正在恢复登录...".to_string()
                });
                let result = services.login_jm(&saved_username, &saved_password).await;
                ui_handle.with_mut(|state| match result {
                    Ok(profile) => {
                        state.jm_profile = Some(profile);
                        state.status = "已自动恢复 JM 登录。".to_string();
                    }
                    Err(err) => {
                        state.status = format!("自动恢复 JM 登录失败：{err}");
                    }
                });
            }
        });
    }

    let status = ui.read().status.clone();
    let loading = ui.read().loading;
    let browse_tab = ui.read().browse_tab;
    let search_query = ui.read().search_query.clone();
    let jm_username = ui.read().jm_username.clone();
    let jm_password = ui.read().jm_password.clone();
    let jm_profile = ui.read().jm_profile.clone();
    let weekly_categories = ui.read().weekly_categories.clone();
    let weekly_types = ui.read().weekly_types.clone();
    let selected_weekly_category = ui.read().selected_weekly_category.clone();
    let selected_weekly_type = ui.read().selected_weekly_type.clone();
    let search_results = ui.read().search_results.clone();
    let selected_comic = ui.read().selected_comic.clone();
    let downloads = ui.read().downloads.clone();
    let library = ui.read().library.clone();
    let library_sort = ui.read().library_sort;
    let mut sorted_library = library.clone();
    sorted_library.sort_by(|a, b| match library_sort {
        crate::state::LibrarySort::DownloadDate => b.download_time.cmp(&a.download_time),
        crate::state::LibrarySort::UpdateDate => b.update_time.cmp(&a.update_time),
        crate::state::LibrarySort::Title => a
            .comic
            .title
            .to_lowercase()
            .cmp(&b.comic.title.to_lowercase()),
        crate::state::LibrarySort::Author => a
            .comic
            .author
            .to_lowercase()
            .cmp(&b.comic.author.to_lowercase()),
    });
    let reader = ui.read().reader.clone();
    let reader_fullscreen = ui.read().reader_fullscreen;
    let site_tab = ui.read().site_tab;
    let workspace_tab = ui.read().workspace_tab;
    let download_panel_tab = ui.read().download_panel_tab;

    rsx! {
        div {
            style: "position:relative; display:flex; height:100vh; background:#f5f4ef; color:#1c1c16; font-family:'SF Pro Text','PingFang SC','Microsoft YaHei',sans-serif;",

            div {
                style: "flex:1; display:flex; flex-direction:column; min-width:0;",

                div {
                    style: "display:flex; align-items:center; gap:12px; padding:18px 24px; background:linear-gradient(135deg,#f8efe2,#efe7d6); border-bottom:1px solid #d7d2c6;",
                    h1 {
                        style: "margin:0; font-size:28px; font-weight:800; letter-spacing:0.04em;",
                        "Hmanga"
                    }
                    span {
                        style: "padding:4px 10px; border-radius:999px; background:#d96f32; color:white; font-size:12px; font-weight:700;",
                        "minimal"
                    }
                }

                div {
                    style: "display:flex; gap:8px; padding:16px; border-bottom:1px solid #d7d2c6;",
                    {workspace_button(workspace_tab == WorkspaceTab::Downloads, "浏览", {
                        let ui_handle = ui;
                        move |_| {
                            let mut ui_handle = ui_handle;
                            ui_handle.with_mut(|state| state.workspace_tab = WorkspaceTab::Downloads);
                        }
                    })}
                    {workspace_button(workspace_tab == WorkspaceTab::Library, "本地书架", {
                        let ui_handle = ui;
                        move |_| {
                            let mut ui_handle = ui_handle;
                            ui_handle.with_mut(|state| state.workspace_tab = WorkspaceTab::Library);
                        }
                    })}
                    {workspace_button(workspace_tab == WorkspaceTab::Settings, "设置", {
                        let ui_handle = ui;
                        move |_| {
                            let mut ui_handle = ui_handle;
                            ui_handle.with_mut(|state| state.workspace_tab = WorkspaceTab::Settings);
                        }
                    })}
                }

                if workspace_tab == WorkspaceTab::Downloads {
                    div {
                        style: "display:flex; flex-direction:column; gap:12px; padding:16px 24px; background:#fffdf8; border-bottom:1px solid #ebe4d8;",
                        div { style: "display:flex; align-items:center; gap:12px;",
                            div { style: "display:flex; gap:8px;",
                                {site_button(site_tab == SiteTab::Aggregate, "聚合", {
                                    let ui_handle = ui;
                                    move |_| {
                                        let mut ui_handle = ui_handle;
                                        ui_handle.with_mut(|state| state.site_tab = SiteTab::Aggregate);
                                    }
                                })}
                                {site_button(site_tab == SiteTab::Jm, "JM", {
                                    let ui_handle = ui;
                                    move |_| {
                                        let mut ui_handle = ui_handle;
                                        ui_handle.with_mut(|state| state.site_tab = SiteTab::Jm);
                                    }
                                })}
                            }
                            div { style: "display:flex; gap:8px; margin-left:auto;",
                                {subtab_button(browse_tab == BrowseTab::Search, "搜索", {
                                    let ui_handle = ui;
                                    move |_| {
                                        let mut ui_handle = ui_handle;
                                        ui_handle.with_mut(|state| state.set_browse_tab(BrowseTab::Search));
                                    }
                                })}
                                {subtab_button(browse_tab == BrowseTab::Favorites, "收藏夹", {
                                    let ui_handle = ui;
                                    let services = services.read().clone();
                                    move |_| {
                                        let mut ui_handle = ui_handle;
                                        ui_handle.with_mut(|state| {
                                            state.site_tab = SiteTab::Jm;
                                            state.set_browse_tab(BrowseTab::Favorites);
                                            state.loading = true;
                                            state.status = "加载收藏夹...".to_string();
                                        });
                                        let services = services.clone();
                                        spawn(async move {
                                            let result = services.get_jm_favorites_page(1).await;
                                            ui_handle.with_mut(|state| {
                                                state.loading = false;
                                                match result {
                                                    Ok(page) => {
                                                        state.search_results = page.comics;
                                                        state.favorites_page = page.current_page;
                                                        state.favorites_total_pages = page.total_pages;
                                                        state.selected_comic = None;
                                                        state.status = format!("收藏夹第 {} / {} 页，共 {} 项。", state.favorites_page, state.favorites_total_pages, state.search_results.len());
                                                    }
                                                    Err(err) => state.status = err,
                                                }
                                            });
                                        });
                                    }
                                })}
                                {subtab_button(browse_tab == BrowseTab::Weekly, "每周必看", {
                                    let ui_handle = ui;
                                    let services = services.read().clone();
                                    move |_| {
                                        let mut ui_handle = ui_handle;
                                        ui_handle.with_mut(|state| {
                                            state.site_tab = SiteTab::Jm;
                                            state.set_browse_tab(BrowseTab::Weekly);
                                            state.loading = true;
                                            state.status = "加载每周必看...".to_string();
                                        });
                                        let services = services.clone();
                                        spawn(async move {
                                            match services.get_jm_weekly_info().await {
                                                Ok(info) => {
                                                    let categories = info.categories.iter().map(|item| crate::state::BrowseFilter {
                                                        id: item.id.clone(),
                                                        label: item.title.clone(),
                                                    }).collect::<Vec<_>>();
                                                    let types = info.types.iter().map(|item| crate::state::BrowseFilter {
                                                        id: item.id.clone(),
                                                        label: item.title.clone(),
                                                    }).collect::<Vec<_>>();
                                                    let category_id = categories.first().map(|item| item.id.clone());
                                                    let type_id = types.first().map(|item| item.id.clone());
                                                    let weekly = if let (Some(category_id), Some(type_id)) = (category_id.clone(), type_id.clone()) {
                                                        services.get_jm_weekly(&category_id, &type_id).await
                                                    } else {
                                                        Ok(Vec::new())
                                                    };
                                                    ui_handle.with_mut(|state| {
                                                        state.loading = false;
                                                        state.weekly_categories = categories;
                                                        state.weekly_types = types;
                                                        state.selected_weekly_category = category_id;
                                                        state.selected_weekly_type = type_id;
                                                        match weekly {
                                                            Ok(comics) => {
                                                                state.search_results = comics;
                                                                state.selected_comic = None;
                                                                state.status = format!("每周必看已加载，共 {} 项。", state.search_results.len());
                                                            }
                                                            Err(err) => state.status = err,
                                                        }
                                                    });
                                                }
                                                Err(err) => ui_handle.with_mut(|state| {
                                                    state.loading = false;
                                                    state.status = err;
                                                }),
                                            }
                                        });
                                    }
                                })}
                            }
                            if browse_tab == BrowseTab::Favorites && !search_results.is_empty() {
                                button {
                                    style: "padding:10px 14px; border:none; border-radius:12px; background:#d96f32; color:white; font-weight:700; cursor:pointer;",
                                    onclick: move |_| {
                                        let services = services.read().clone();
                                        let mut ui_handle = ui;
                                        let favorites = ui_handle.read().search_results.clone();
                                        let favorites_interval =
                                            ui_handle.read().settings_config.download_all_favorites_interval_sec;
                                        ui_handle.with_mut(|state| {
                                            state.status = format!("开始批量下载收藏夹，共 {} 部漫画。", favorites.len());
                                        });
                                        spawn(async move {
                                            let total = favorites.len();
                                            for (index, favorite) in favorites.into_iter().enumerate() {
                                                let comic = match services.load_jm_comic(&favorite.id).await {
                                                    Ok(comic) => comic,
                                                    Err(err) => {
                                                        ui_handle.with_mut(|state| state.status = err);
                                                        continue;
                                                    }
                                                };
                                                for chapter in comic.chapters.clone() {
                                                    enqueue_download(&mut ui_handle, services.clone(), comic.clone(), chapter);
                                                }
                                                if favorites_interval > 0 && index + 1 < total {
                                                    ui_handle.with_mut(|state| {
                                                        state.status = format!(
                                                            "收藏夹批量下载已处理 {}/{}，等待 {} 秒后继续...",
                                                            index + 1,
                                                            total,
                                                            favorites_interval
                                                        );
                                                    });
                                                    tokio::time::sleep(std::time::Duration::from_secs(favorites_interval)).await;
                                                }
                                            }
                                            ui_handle.with_mut(|state| {
                                                state.status = format!("收藏夹批量下载任务已加入队列，共 {} 部漫画。", total);
                                            });
                                        });
                                    },
                                    "下载全部收藏"
                                }
                            }
                        }
                        if site_tab == SiteTab::Jm {
                            div { style: "display:flex; align-items:center; gap:10px; padding:12px 14px; border-radius:14px; background:#f7f1e6; border:1px solid #e6dac5;",
                                if let Some(profile) = jm_profile.clone() {
                                    div { style: "display:flex; align-items:center; gap:10px; width:100%;",
                                        div { style: "font-weight:700;", "{profile.username}" }
                                        div { style: "color:#7a7366; font-size:13px;", "{profile.level_name}" }
                                        div { style: "margin-left:auto; color:#7a7366; font-size:13px;", "收藏 {profile.favorites_count}/{profile.favorites_max}" }
                                        if let Some(coin) = profile.extra.get("coin") {
                                            div { style: "color:#7a7366; font-size:13px;", "金币 {coin}" }
                                        }
                                    }
                                } else {
                                    input {
                                        style: "width:140px; padding:10px 12px; border-radius:10px; border:1px solid #d8cfbe; background:white;",
                                        value: "{jm_username}",
                                        placeholder: "JM 用户名",
                                        oninput: move |event| ui.with_mut(|state| state.jm_username = event.value())
                                    }
                                    input {
                                        r#type: "password",
                                        style: "width:160px; padding:10px 12px; border-radius:10px; border:1px solid #d8cfbe; background:white;",
                                        value: "{jm_password}",
                                        placeholder: "JM 密码",
                                        oninput: move |event| ui.with_mut(|state| state.jm_password = event.value())
                                    }
                                    button {
                                        style: button_style(true),
                                        onclick: move |_| {
                                            let services = services.read().clone();
                                            let mut ui_handle = ui;
                                            let username = ui_handle.read().jm_username.clone();
                                            let password = ui_handle.read().jm_password.clone();
                                            if username.is_empty() || password.is_empty() {
                                                ui_handle.with_mut(|state| state.status = "请输入 JM 账号和密码。".to_string());
                                                return;
                                            }
                                            ui_handle.with_mut(|state| {
                                                state.loading = true;
                                                state.status = "登录 JM...".to_string();
                                            });
                                            spawn(async move {
                                                let result = services.login_jm(&username, &password).await;
                                                ui_handle.with_mut(|state| {
                                                    state.loading = false;
                                                    match result {
                                                        Ok(profile) => {
                                                            let _ = services
                                                                .save_jm_credentials(
                                                                    &username,
                                                                    &password,
                                                                );
                                                            state.jm_profile = Some(profile);
                                                            state.status = "JM 登录成功。".to_string();
                                                        }
                                                        Err(err) => state.status = err,
                                                    }
                                                });
                                            });
                                        },
                                        "登录"
                                    }
                                }
                            }
                        }
                        if browse_tab == BrowseTab::Search {
                            div { style: "display:flex; align-items:center; gap:12px;",
                                input {
                                    style: "flex:1; padding:12px 14px; border-radius:12px; border:1px solid #d8cfbe; background:white; font-size:14px;",
                                    value: "{search_query}",
                                    placeholder: if site_tab == SiteTab::Aggregate { "在所有已启用源里搜索（当前最小版实际走 JM）" } else { "搜索 JM 漫画或直接输入 jm 号" },
                                    oninput: move |event| ui.with_mut(|state| state.search_query = event.value())
                                }
                                button {
                                    style: button_style(true),
                                    disabled: loading,
                                    onclick: move |_| {
                                        let services = services.read().clone();
                                        let mut ui_handle = ui;
                                        let query = ui_handle.read().search_query.trim().to_string();
                                        let site = ui_handle.read().site_tab;
                                        if query.is_empty() {
                                            ui_handle.with_mut(|state| state.status = "请输入关键词。".to_string());
                                            return;
                                        }
                                        ui_handle.with_mut(|state| {
                                            state.loading = true;
                                            state.status = "搜索中...".to_string();
                                        });
                                        spawn(async move {
                                            let result = match site {
                                                SiteTab::Aggregate => services.search_aggregate(&query).await,
                                                SiteTab::Jm => services.search_jm(&query).await,
                                            };
                                            ui_handle.with_mut(|state| {
                                                state.loading = false;
                                                match result {
                                                    Ok(comics) => {
                                                        state.search_results = comics;
                                                        state.selected_comic = None;
                                                        state.status = format!("搜索完成，共 {} 部漫画。", state.search_results.len());
                                                    }
                                                    Err(err) => state.status = err,
                                                }
                                            });
                                        });
                                    },
                                    if loading { "处理中..." } else { "搜索" }
                                }
                            }
                        }
                        if browse_tab == BrowseTab::Favorites {
                            div { style: "display:flex; align-items:center; gap:12px;",
                                button {
                                    style: button_style(ui.read().favorites_page > 1),
                                    disabled: ui.read().favorites_page <= 1,
                                    onclick: move |_| {
                                        let services = services.read().clone();
                                        let mut ui_handle = ui;
                                        let current_page = ui_handle.read().favorites_page;
                                        if current_page <= 1 {
                                            return;
                                        }
                                        let next_page = current_page - 1;
                                        ui_handle.with_mut(|state| {
                                            state.loading = true;
                                            state.status = format!("加载收藏夹第 {next_page} 页...");
                                        });
                                        spawn(async move {
                                            let result = services.get_jm_favorites_page(next_page).await;
                                            ui_handle.with_mut(|state| {
                                                state.loading = false;
                                                match result {
                                                    Ok(page) => {
                                                        state.search_results = page.comics;
                                                        state.favorites_page = page.current_page;
                                                        state.favorites_total_pages = page.total_pages;
                                                        state.selected_comic = None;
                                                        state.status = format!("收藏夹第 {} / {} 页", state.favorites_page, state.favorites_total_pages);
                                                    }
                                                    Err(err) => state.status = err,
                                                }
                                            });
                                        });
                                    },
                                    "上一页"
                                }
                                div { style: "color:#7a7366; font-size:13px;", "收藏夹第 {ui.read().favorites_page} / {ui.read().favorites_total_pages} 页" }
                                button {
                                    style: button_style(ui.read().favorites_page < ui.read().favorites_total_pages),
                                    disabled: ui.read().favorites_page >= ui.read().favorites_total_pages,
                                    onclick: move |_| {
                                        let services = services.read().clone();
                                        let mut ui_handle = ui;
                                        let current_page = ui_handle.read().favorites_page;
                                        let total_pages = ui_handle.read().favorites_total_pages;
                                        if current_page >= total_pages {
                                            return;
                                        }
                                        let next_page = current_page + 1;
                                        ui_handle.with_mut(|state| {
                                            state.loading = true;
                                            state.status = format!("加载收藏夹第 {next_page} 页...");
                                        });
                                        spawn(async move {
                                            let result = services.get_jm_favorites_page(next_page).await;
                                            ui_handle.with_mut(|state| {
                                                state.loading = false;
                                                match result {
                                                    Ok(page) => {
                                                        state.search_results = page.comics;
                                                        state.favorites_page = page.current_page;
                                                        state.favorites_total_pages = page.total_pages;
                                                        state.selected_comic = None;
                                                        state.status = format!("收藏夹第 {} / {} 页", state.favorites_page, state.favorites_total_pages);
                                                    }
                                                    Err(err) => state.status = err,
                                                }
                                            });
                                        });
                                    },
                                    "下一页"
                                }
                            }
                        }
                        if browse_tab == BrowseTab::Weekly {
                            div { style: "padding:14px 20px; border-bottom:1px solid #ebe4d8; background:#faf8f4;",
                                div { style: "font-size:13px; font-weight:800; color:#8a6f2f; margin-bottom:10px;", "每周必看" }
                                if !weekly_types.is_empty() {
                                    div { style: "display:flex; align-items:center; gap:8px; flex-wrap:wrap;",
                                        span { style: "font-size:12px; font-weight:700; color:#6a5f4e; min-width:32px;", "类型" }
                                        for weekly_type in weekly_types {
                                            {subtab_button(selected_weekly_type.as_deref() == Some(weekly_type.id.as_str()), weekly_type.label.clone(), {
                                                let ui_handle = ui;
                                                let services = services.read().clone();
                                                let type_id = weekly_type.id.clone();
                                                let selected_category = selected_weekly_category.clone().unwrap_or_else(|| "0".to_string());
                                                move |_| {
                                                    let mut ui_handle = ui_handle;
                                                    ui_handle.with_mut(|state| {
                                                        state.loading = true;
                                                        state.selected_weekly_type = Some(type_id.clone());
                                                        state.status = "切换每周类型...".to_string();
                                                    });
                                                    let services = services.clone();
                                                    let type_id = type_id.clone();
                                                    let selected_category = selected_category.clone();
                                                    spawn(async move {
                                                        let result = services.get_jm_weekly(&selected_category, &type_id).await;
                                                        ui_handle.with_mut(|state| {
                                                            state.loading = false;
                                                            match result {
                                                                Ok(comics) => {
                                                                    state.search_results = comics;
                                                                    state.selected_comic = None;
                                                                    state.status = format!("每周必看已切换，共 {} 项。", state.search_results.len());
                                                                }
                                                                Err(err) => state.status = err,
                                                            }
                                                        });
                                                    });
                                                }
                                            })}
                                        }
                                    }
                                }
                            }
                        }
                    }

                    div {
                        style: "flex:1; display:flex; min-height:0;",

                        div {
                            style: "flex:1; display:flex; min-height:0;",
                            div {
                                style: "width:42%; min-width:280px; border-right:1px solid #ebe4d8; overflow:auto; padding:18px 20px;",
                                h2 { style: section_title_style(), "浏览结果" }
                                if search_results.is_empty() {
                                    {empty_block("还没有搜索结果")}
                                } else {
                                    for comic in search_results {
                                        {comic_row(comic, Rc::new(move |comic_id| {
                                            let services = services.read().clone();
                                            let mut ui_handle = ui;
                                            spawn(async move {
                                                ui_handle.with_mut(|state| state.status = "加载漫画详情...".to_string());
                                                match services.load_jm_comic(&comic_id).await {
                                                    Ok(comic) => ui_handle.with_mut(|state| {
                                                        state.selected_comic = Some(comic);
                                                        state.status = "漫画详情已加载。".to_string();
                                                    }),
                                                    Err(err) => ui_handle.with_mut(|state| state.status = err),
                                                }
                                            });
                                        }))}
                                    }
                                }
                            }

                            div {
                                style: "flex:1; overflow:auto; padding:18px 20px; background:#fffdfa;",
                                h2 { style: section_title_style(), "章节与下载" }
                                if let Some(comic) = selected_comic {
                                    div {
                                        style: "display:flex; flex-direction:column; gap:14px;",
                                        div {
                                            style: "padding:16px; border-radius:16px; background:white; border:1px solid #ebe4d8; box-shadow:0 8px 24px rgba(60,40,10,0.04);",
                                            div {
                                                style: "display:flex; gap:16px; align-items:flex-start;",
                                                if !comic.cover_url.is_empty() {
                                                    img {
                                                        style: "width:112px; min-width:112px; aspect-ratio:3/4; object-fit:cover; border-radius:14px; border:1px solid #e9e0d2; background:#f6f1e8;",
                                                        src: "{comic.cover_url}"
                                                    }
                                                }
                                                div { style: "flex:1; min-width:0; display:flex; flex-direction:column; gap:8px;",
                                                    h3 { style: "margin:0; font-size:22px; line-height:1.3;", "{comic.title}" }
                                                    div { style: "display:flex; gap:8px; flex-wrap:wrap;",
                                                        span {
                                                            style: "display:inline-flex; align-items:center; gap:6px; padding:5px 9px; border-radius:999px; background:#f6f1e8; color:#6a5f4e; font-size:12px;",
                                                            strong { style: "font-weight:800;", "来源" }
                                                            span { "{comic.source.to_uppercase()}" }
                                                        }
                                                        span {
                                                            style: "display:inline-flex; align-items:center; gap:6px; padding:5px 9px; border-radius:999px; background:#f6f1e8; color:#6a5f4e; font-size:12px;",
                                                            strong { style: "font-weight:800;", "作者" }
                                                            span { "{comic.author}" }
                                                        }
                                                        span {
                                                            style: "display:inline-flex; align-items:center; gap:6px; padding:5px 9px; border-radius:999px; background:#f6f1e8; color:#6a5f4e; font-size:12px;",
                                                            strong { style: "font-weight:800;", "章节" }
                                                            span { "{comic.chapters.len()}" }
                                                        }
                                                        if let Some(total_views) = comic.extra.get("total_views") {
                                                            span {
                                                                style: "display:inline-flex; align-items:center; gap:6px; padding:5px 9px; border-radius:999px; background:#f6f1e8; color:#6a5f4e; font-size:12px;",
                                                                strong { style: "font-weight:800;", "浏览" }
                                                                span { "{total_views}" }
                                                            }
                                                        }
                                                        if let Some(likes) = comic.extra.get("likes") {
                                                            span {
                                                                style: "display:inline-flex; align-items:center; gap:6px; padding:5px 9px; border-radius:999px; background:#f6f1e8; color:#6a5f4e; font-size:12px;",
                                                                strong { style: "font-weight:800;", "点赞" }
                                                                span { "{likes}" }
                                                            }
                                                        }
                                                        if let Some(comment_total) = comic.extra.get("comment_total") {
                                                            span {
                                                                style: "display:inline-flex; align-items:center; gap:6px; padding:5px 9px; border-radius:999px; background:#f6f1e8; color:#6a5f4e; font-size:12px;",
                                                                strong { style: "font-weight:800;", "评论" }
                                                                span { "{comment_total}" }
                                                            }
                                                        }
                                                        if let Some(is_favorite) = comic.extra.get("is_favorite") {
                                                            if is_favorite == "true" {
                                                                span {
                                                                    style: "display:inline-flex; align-items:center; gap:6px; padding:5px 9px; border-radius:999px; background:#f6f1e8; color:#6a5f4e; font-size:12px;",
                                                                    strong { style: "font-weight:800;", "状态" }
                                                                    span { "已收藏" }
                                                                }
                                                            }
                                                        }
                                                        if let Some(liked) = comic.extra.get("liked") {
                                                            if liked == "true" {
                                                                span {
                                                                    style: "display:inline-flex; align-items:center; gap:6px; padding:5px 9px; border-radius:999px; background:#f6f1e8; color:#6a5f4e; font-size:12px;",
                                                                    strong { style: "font-weight:800;", "偏好" }
                                                                    span { "已点赞" }
                                                                }
                                                            }
                                                        }
                                                    }
                                                    if !comic.description.is_empty() {
                                                        p { style: "margin:0; color:#665f52; line-height:1.6;", "{comic.description}" }
                                                    }
                                                    if !comic.tags.is_empty() {
                                                        div { style: "display:flex; gap:6px; flex-wrap:wrap;",
                                                            for tag in comic.tags.iter().take(8) {
                                                                span {
                                                                    style: "padding:4px 8px; border-radius:999px; background:#f0eadc; color:#5f5648; font-size:12px;",
                                                                    "{tag}"
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                        div {
                                            style: "display:flex; flex-direction:column; gap:10px;",
                                            for chapter in comic.chapters.clone() {
                                                {download_chapter_row(comic.clone(), chapter, library.clone(), services.read().clone(), ui)}
                                            }
                                        }
                                    }
                                } else {
                                    {empty_block("先从左侧选择一部漫画。")}
                                }
                            }
                        }

                        div {
                            style: "width:420px; display:flex; flex-direction:column; min-width:320px; background:#faf7f0; border-left:1px solid #d7d2c6;",
                            div {
                                style: "display:flex; gap:8px; padding:16px; border-bottom:1px solid #d7d2c6;",
                                {subtab_button(download_panel_tab == DownloadPanelTab::Queue, "下载队列", {
                                    let ui_handle = ui;
                                    move |_| {
                                        let mut ui_handle = ui_handle;
                                        ui_handle.with_mut(|state| state.download_panel_tab = DownloadPanelTab::Queue);
                                    }
                                })}
                                {subtab_button(download_panel_tab == DownloadPanelTab::Preview, "预览", {
                                    let ui_handle = ui;
                                    move |_| {
                                        let mut ui_handle = ui_handle;
                                        ui_handle.with_mut(|state| state.download_panel_tab = DownloadPanelTab::Preview);
                                    }
                                })}
                            }
                            div {
                                style: "flex:1; overflow:auto; padding:16px;",
                                if download_panel_tab == DownloadPanelTab::Queue {
                                    div {
                                        h2 { style: section_title_style(), "下载队列" }
                                        if downloads.is_empty() {
                                            {empty_block("还没有下载任务。")}
                                        } else {
                                            for row in downloads {
                                                {download_queue_row(row, library.clone(), services.read().clone(), ui)}
                                            }
                                        }
                                    }
                                } else {
                                    div {
                                        h2 { style: section_title_style(), "预览" }
                                        {reader_panel(reader.clone(), ui, "下载完成后可直接在这里预览。", true)}
                                    }
                                }
                            }
                        }
                    }
                } else if workspace_tab == WorkspaceTab::Library {
                    div {
                        style: "flex:1; display:flex; min-height:0;",
                        div {
                            style: "flex:1; overflow:auto; padding:18px 20px; border-right:1px solid #d7d2c6;",
                            div { style: "display:flex; align-items:center; gap:12px; margin-bottom:14px;",
                                h2 { style: "margin:0; font-size:16px; font-weight:800; letter-spacing:0.02em;", "本地书架" }
                                select {
                                    style: "margin-left:auto; padding:8px 12px; border-radius:10px; border:1px solid #d8cfbe; background:white;",
                                    onchange: move |event| ui.with_mut(|state| {
                                        state.library_sort = match event.value().as_str() {
                                            "download" => crate::state::LibrarySort::DownloadDate,
                                            "update" => crate::state::LibrarySort::UpdateDate,
                                            "title" => crate::state::LibrarySort::Title,
                                            "author" => crate::state::LibrarySort::Author,
                                            _ => crate::state::LibrarySort::DownloadDate,
                                        };
                                    }),
                                    option { value: "download", if matches!(ui.read().library_sort, crate::state::LibrarySort::DownloadDate) { "selected" } else { "" }, "下载日期" }
                                    option { value: "update", if matches!(ui.read().library_sort, crate::state::LibrarySort::UpdateDate) { "selected" } else { "" }, "更新日期" }
                                    option { value: "title", if matches!(ui.read().library_sort, crate::state::LibrarySort::Title) { "selected" } else { "" }, "标题" }
                                    option { value: "author", if matches!(ui.read().library_sort, crate::state::LibrarySort::Author) { "selected" } else { "" }, "作者" }
                                }
                                button {
                                    style: "padding:8px 12px; border:none; border-radius:10px; background:#6b5b3d; color:white; font-weight:700; cursor:pointer;",
                                    onclick: move |_| {
                                        let services = services.read().clone();
                                        let mut ui_handle = ui;
                                        ui_handle.with_mut(|state| state.status = "正在重新扫描书架...".to_string());
                                        spawn(async move {
                                            match services.read_library() {
                                                Ok(library) => {
                                                    let count = library.len();
                                                    ui_handle.with_mut(|state| {
                                                        state.library = library;
                                                        state.status = format!("书架已重新扫描，共 {} 本漫画。", count);
                                                    });
                                                }
                                                Err(err) => ui_handle.with_mut(|state| state.status = err),
                                            }
                                        });
                                    },
                                    "重新扫描"
                                }
                                button {
                                    style: "padding:8px 12px; border:none; border-radius:10px; background:#8a6f2f; color:white; font-weight:700; cursor:pointer;",
                                    onclick: move |_| {
                                        let services = services.read().clone();
                                        let mut ui_handle = ui;
                                        ui_handle.with_mut(|state| state.status = "正在扫描库存并追加缺失章节...".to_string());
                                        spawn(async move {
                                            let result = services.update_library_queue().await;
                                            match result {
                                                Ok(queue) => {
                                                    let total = queue.len();
                                                    for (comic, chapter) in queue {
                                                        enqueue_download(&mut ui_handle, services.clone(), comic, chapter);
                                                    }
                                                    ui_handle.with_mut(|state| {
                                                        state.status = if total == 0 {
                                                            "库存已是最新，无需补下。".to_string()
                                                        } else {
                                                            format!("库存扫描完成，已追加 {} 个章节到下载队列。", total)
                                                        };
                                                    });
                                                }
                                                Err(err) => ui_handle.with_mut(|state| state.status = err),
                                            }
                                        });
                                    },
                                    "更新库存"
                                }
                            }
                            if library.is_empty() {
                                {empty_block("下载目录还没有漫画。")}
                            } else {
                                for item in sorted_library {
                                    {local_comic_card(item, Rc::new({
                                        let ui_handle = ui;
                                        move |chapter| {
                                            let mut ui = ui_handle;
                                            ui.with_mut(|state| {
                                                state.open_reader(ReaderState {
                                                    title: format!("{} / {}", chapter.comic_title, chapter.chapter_title),
                                                    pages: chapter.pages.iter().map(|path| to_browser_src(path)).collect(),
                                                    current_index: 0,
                                                    source_dir: Some(chapter.chapter_dir.clone()),
                                                });
                                                state.status = format!("打开阅读器：{}", state.reader.title);
                                            });
                                        }
                                    }), Rc::new({
                                        let ui_handle = ui;
                                        let services = services.read().clone();
                                        move |chapter| {
                                            let mut ui_handle = ui_handle;
                                            let services = services.clone();
                                            spawn(async move {
                                                let result = services.export_local_chapter_cbz(&chapter);
                                                ui_handle.with_mut(|state| {
                                                    state.status = match result {
                                                        Ok(path) => format!("已导出 CBZ：{}", path.to_string_lossy()),
                                                        Err(err) => err,
                                                    };
                                                });
                                            });
                                        }
                                    }), Rc::new(move |comic_dir| {
                                        let services = services.read().clone();
                                        let mut ui_handle = ui;
                                        spawn(async move {
                                            let result = services.delete_local_comic(&comic_dir);
                                            let library = services.read_library().unwrap_or_default();
                                            ui_handle.with_mut(|state| {
                                                state.library = library;
                                                if state.reader.title.starts_with("已删除") {
                                                    state.reader = ReaderState::default();
                                                }
                                                state.status = result.err().unwrap_or_else(|| "已删除本地漫画。".to_string());
                                            });
                                        });
                                    }))}
                                }
                            }
                        }
                        div {
                            style: "width:460px; min-width:320px; overflow:auto; padding:18px 20px; background:#faf7f0;",
                            h2 { style: section_title_style(), "阅读器" }
                            {reader_panel(reader.clone(), ui, "从左侧本地漫画列表选择章节后在这里阅读。", true)}
                        }
                    }
                } else {
                    div {
                        style: "flex:1; overflow:auto; padding:24px; background:#fffdfa;",
                        h2 { style: section_title_style(), "设置" }
                        div {
                            style: "display:flex; flex-direction:column; gap:14px; max-width:760px; padding:18px; border-radius:18px; background:white; border:1px solid #ebe4d8;",
                            div {
                                style: "display:grid; grid-template-columns:160px 1fr; gap:12px; align-items:center;",
                                label { style: "font-weight:700;", "下载目录" }
                                input {
                                    style: "padding:12px 14px; border-radius:12px; border:1px solid #d8cfbe; background:white;",
                                    value: "{ui.read().settings_config.download_dir.to_string_lossy()}",
                                    oninput: move |event| ui.with_mut(|state| state.settings_config.download_dir = std::path::PathBuf::from(event.value()))
                                }
                            }
                            div {
                                style: "display:grid; grid-template-columns:160px 1fr; gap:12px; align-items:center;",
                                label { style: "font-weight:700;", "导出目录" }
                                input {
                                    style: "padding:12px 14px; border-radius:12px; border:1px solid #d8cfbe; background:white;",
                                    value: "{ui.read().settings_config.export_dir.to_string_lossy()}",
                                    oninput: move |event| ui.with_mut(|state| state.settings_config.export_dir = std::path::PathBuf::from(event.value()))
                                }
                            }
                            div {
                                style: "display:grid; grid-template-columns:160px 1fr; gap:12px; align-items:center;",
                                label { style: "font-weight:700;", "下载格式" }
                                select {
                                    style: "padding:12px 14px; border-radius:12px; border:1px solid #d8cfbe; background:white;",
                                    onchange: move |event| ui.with_mut(|state| state.settings_config.download_format = event.value()),
                                    option { value: "webp", if ui.read().settings_config.download_format == "webp" { "selected" } else { "" }, "webp" }
                                    option { value: "jpg", if ui.read().settings_config.download_format == "jpg" { "selected" } else { "" }, "jpg" }
                                    option { value: "png", if ui.read().settings_config.download_format == "png" { "selected" } else { "" }, "png" }
                                }
                            }
                            div {
                                style: "display:grid; grid-template-columns:160px 1fr; gap:12px; align-items:center;",
                                label { style: "font-weight:700;", "下载封面" }
                                input {
                                    r#type: "checkbox",
                                    checked: "{ui.read().settings_config.should_download_cover}",
                                    onchange: move |event| ui.with_mut(|state| state.settings_config.should_download_cover = event.value() == "true"),
                                }
                            }
                            div {
                                style: "display:grid; grid-template-columns:160px 1fr; gap:12px; align-items:center;",
                                label { style: "font-weight:700;", "章节并发数" }
                                input {
                                    r#type: "number",
                                    min: "1",
                                    style: "padding:12px 14px; border-radius:12px; border:1px solid #d8cfbe; background:white;",
                                    value: "{ui.read().settings_config.chapter_concurrency}",
                                    oninput: move |event| ui.with_mut(|state| {
                                        state.settings_config.chapter_concurrency = event.value().parse::<usize>().unwrap_or(1).max(1);
                                    })
                                }
                            }
                            div {
                                style: "display:grid; grid-template-columns:160px 1fr; gap:12px; align-items:center;",
                                label { style: "font-weight:700;", "章节完成后休息秒数" }
                                input {
                                    r#type: "number",
                                    min: "0",
                                    style: "padding:12px 14px; border-radius:12px; border:1px solid #d8cfbe; background:white;",
                                    value: "{ui.read().settings_config.chapter_download_interval_sec}",
                                    oninput: move |event| ui.with_mut(|state| {
                                        state.settings_config.chapter_download_interval_sec = event.value().parse::<u64>().unwrap_or(0);
                                    })
                                }
                            }
                            div {
                                style: "display:grid; grid-template-columns:160px 1fr; gap:12px; align-items:center;",
                                label { style: "font-weight:700;", "图片并发数" }
                                input {
                                    r#type: "number",
                                    min: "1",
                                    style: "padding:12px 14px; border-radius:12px; border:1px solid #d8cfbe; background:white;",
                                    value: "{ui.read().settings_config.image_concurrency}",
                                    oninput: move |event| ui.with_mut(|state| {
                                        state.settings_config.image_concurrency = event.value().parse::<usize>().unwrap_or(1).max(1);
                                    })
                                }
                            }
                            div {
                                style: "display:grid; grid-template-columns:160px 1fr; gap:12px; align-items:center;",
                                label { style: "font-weight:700;", "每张图片后休息秒数" }
                                input {
                                    r#type: "number",
                                    min: "0",
                                    style: "padding:12px 14px; border-radius:12px; border:1px solid #d8cfbe; background:white;",
                                    value: "{ui.read().settings_config.image_download_interval_sec}",
                                    oninput: move |event| ui.with_mut(|state| {
                                        state.settings_config.image_download_interval_sec = event.value().parse::<u64>().unwrap_or(0);
                                    })
                                }
                            }
                            div {
                                style: "display:grid; grid-template-columns:160px 1fr; gap:12px; align-items:center;",
                                label { style: "font-weight:700;", "收藏夹批量间隔秒数" }
                                input {
                                    r#type: "number",
                                    min: "0",
                                    style: "padding:12px 14px; border-radius:12px; border:1px solid #d8cfbe; background:white;",
                                    value: "{ui.read().settings_config.download_all_favorites_interval_sec}",
                                    oninput: move |event| ui.with_mut(|state| {
                                        state.settings_config.download_all_favorites_interval_sec = event.value().parse::<u64>().unwrap_or(0);
                                    })
                                }
                            }
                            div {
                                style: "display:grid; grid-template-columns:160px 1fr; gap:12px; align-items:center;",
                                label { style: "font-weight:700;", "库存更新间隔秒数" }
                                input {
                                    r#type: "number",
                                    min: "0",
                                    style: "padding:12px 14px; border-radius:12px; border:1px solid #d8cfbe; background:white;",
                                    value: "{ui.read().settings_config.update_downloaded_comics_interval_sec}",
                                    oninput: move |event| ui.with_mut(|state| {
                                        state.settings_config.update_downloaded_comics_interval_sec = event.value().parse::<u64>().unwrap_or(0);
                                    })
                                }
                            }
                            div {
                                style: "display:grid; grid-template-columns:160px 1fr; gap:12px; align-items:center;",
                                label { style: "font-weight:700;", "API 域名" }
                                input {
                                    style: "padding:12px 14px; border-radius:12px; border:1px solid #d8cfbe; background:white;",
                                    value: "{ui.read().settings_config.api_domain}",
                                    oninput: move |event| ui.with_mut(|state| state.settings_config.api_domain = event.value())
                                }
                            }
                            div {
                                style: "display:grid; grid-template-columns:160px 1fr; gap:12px; align-items:center;",
                                label { style: "font-weight:700;", "自定义 API 域名" }
                                input {
                                    style: "padding:12px 14px; border-radius:12px; border:1px solid #d8cfbe; background:white;",
                                    value: "{ui.read().settings_config.custom_api_domain}",
                                    oninput: move |event| ui.with_mut(|state| state.settings_config.custom_api_domain = event.value())
                                }
                            }
                            div {
                                style: "display:grid; grid-template-columns:160px 1fr; gap:12px; align-items:center;",
                                label { style: "font-weight:700;", "HTTP 代理" }
                                input {
                                    style: "padding:12px 14px; border-radius:12px; border:1px solid #d8cfbe; background:white;",
                                    value: "{ui.read().settings_config.proxy.clone().unwrap_or_default()}",
                                    placeholder: "例如 http://127.0.0.1:7890",
                                    oninput: move |event| ui.with_mut(|state| {
                                        let value = event.value();
                                        state.settings_config.proxy = if value.trim().is_empty() {
                                            None
                                        } else {
                                            Some(value)
                                        };
                                    })
                                }
                            }
                            div {
                                style: "display:grid; grid-template-columns:160px 1fr; gap:12px; align-items:center;",
                                label { style: "font-weight:700;", "JM 用户名" }
                                input {
                                    style: "padding:12px 14px; border-radius:12px; border:1px solid #d8cfbe; background:white;",
                                    value: "{ui.read().settings_config.jm_username}",
                                    oninput: move |event| ui.with_mut(|state| state.settings_config.jm_username = event.value())
                                }
                            }
                            div {
                                style: "display:grid; grid-template-columns:160px 1fr; gap:12px; align-items:center;",
                                label { style: "font-weight:700;", "JM 密码" }
                                input {
                                    r#type: "password",
                                    style: "padding:12px 14px; border-radius:12px; border:1px solid #d8cfbe; background:white;",
                                    value: "{ui.read().settings_config.jm_password}",
                                    oninput: move |event| ui.with_mut(|state| state.settings_config.jm_password = event.value())
                                }
                            }
                            div {
                                style: "display:flex; justify-content:flex-end; gap:10px; margin-top:8px;",
                                button {
                                    style: button_style(true),
                                    onclick: move |_| {
                                        let services = services.read().clone();
                                        let mut ui_handle = ui;
                                        let config = ui_handle.read().settings_config.clone();
                                        match services.save_config(&config) {
                                            Ok(()) => ui_handle.with_mut(|state| {
                                                state.jm_username = config.jm_username.clone();
                                                state.jm_password = config.jm_password.clone();
                                                state.status = "设置已保存。".to_string();
                                            }),
                                            Err(err) => ui_handle.with_mut(|state| state.status = err),
                                        }
                                    },
                                    "保存设置"
                                }
                            }
                        }
                    }
                }

                div {
                    style: "padding:12px 24px; border-top:1px solid #ebe4d8; background:#fffdf8; color:#665f52;",
                    div {
                        style: "font-size:13px;",
                        "{status}"
                    }
                    div {
                        style: "display:flex; flex-wrap:wrap; gap:10px 16px; margin-top:10px; font-size:11px; line-height:1.6; color:#8a8477;",
                        span { "Hmanga v0.1.2" }
                        span { "仅供学习、研究与个人归档使用" }
                        span { "本项目与 JM / 18comic 及相关版权方无官方关联" }
                        span { "漫画、封面与站点内容版权归原作者及权利人所有" }
                        span { "请自行承担使用风险并遵守当地法律与平台条款" }
                    }
                }
            }

            if reader_fullscreen && !reader.pages.is_empty() {
                div {
                    style: "position:absolute; inset:0; z-index:50; background:#111; color:#f6f2e8; display:flex; flex-direction:column;",
                    tabindex: 0,
                    onkeydown: move |evt| {
                        match evt.key() {
                            Key::ArrowLeft | Key::ArrowUp => {
                                ui.with_mut(|state| {
                                    if state.reader.current_index > 0 {
                                        state.reader.current_index -= 1;
                                    }
                                });
                            }
                            Key::ArrowRight | Key::ArrowDown => {
                                ui.with_mut(|state| {
                                    if state.reader.current_index + 1 < state.reader.pages.len() {
                                        state.reader.current_index += 1;
                                    }
                                });
                            }
                            Key::Home => {
                                ui.with_mut(|state| {
                                    state.reader.current_index = 0;
                                });
                            }
                            Key::End => {
                                ui.with_mut(|state| {
                                    state.reader.current_index = state.reader.pages.len().saturating_sub(1);
                                });
                            }
                            Key::Escape => {
                                ui.with_mut(|state| state.close_reader_fullscreen());
                            }
                            _ => {}
                        }
                    },
                    div {
                        style: "display:flex; align-items:center; gap:12px; padding:16px 20px; border-bottom:1px solid rgba(255,255,255,0.12);",
                        div { style: "font-size:16px; font-weight:800;", "{reader.title}" }
                        div { style: "margin-left:auto; font-size:12px; color:#888;", "← → 导航 | Home End 首尾页 | Esc 退出" }
                        button {
                            style: "padding:10px 14px; border:none; border-radius:12px; background:#f0eadc; color:#111; font-weight:700; cursor:pointer;",
                            onclick: move |_| ui.with_mut(|state| state.close_reader_fullscreen()),
                            "退出纯净阅读"
                        }
                    }
                    div {
                        style: "flex:1; display:flex; align-items:center; justify-content:center; overflow:auto; padding:24px;",
                        img {
                            style: "max-width:100%; max-height:100%; object-fit:contain; border-radius:8px;",
                            src: "{reader.pages[reader.current_index]}"
                        }
                    }
                    div {
                        style: "display:flex; align-items:center; justify-content:center; gap:16px; padding:12px 20px; border-top:1px solid rgba(255,255,255,0.12);",
                        button {
                            style: button_style(reader.current_index > 0),
                            disabled: reader.current_index == 0,
                            onclick: move |_| ui.with_mut(|state| {
                                if state.reader.current_index > 0 {
                                    state.reader.current_index -= 1;
                                }
                            }),
                            "上一页"
                        }
                        div { style: "font-weight:700; color:#f6f2e8;", "第 {reader.current_index + 1} / {reader.pages.len()} 页" }
                        button {
                            style: button_style(reader.current_index + 1 < reader.pages.len()),
                            disabled: reader.current_index + 1 >= reader.pages.len(),
                            onclick: move |_| ui.with_mut(|state| {
                                if state.reader.current_index + 1 < state.reader.pages.len() {
                                    state.reader.current_index += 1;
                                }
                            }),
                            "下一页"
                        }
                    }
                }
            }
        }
    }
}

fn button_style(enabled: bool) -> &'static str {
    if enabled {
        "padding:10px 14px; border:none; border-radius:12px; background:#d96f32; color:white; font-weight:700; cursor:pointer;"
    } else {
        "padding:10px 14px; border:none; border-radius:12px; background:#d8d0c1; color:#8a8477; font-weight:700; cursor:not-allowed;"
    }
}

fn section_title_style() -> &'static str {
    "margin:0 0 14px 0; font-size:16px; font-weight:800; letter-spacing:0.02em;"
}

fn site_button<F>(on: bool, label: &'static str, onclick: F) -> Element
where
    F: Fn(MouseEvent) + Clone + 'static,
{
    let handler = onclick.clone();
    rsx! {
        button {
            style: if on {
                "padding:10px 16px; border:none; border-radius:999px; background:#1f4d3b; color:white; font-weight:700; cursor:pointer;"
            } else {
                "padding:10px 16px; border:none; border-radius:999px; background:white; color:#1f4d3b; font-weight:700; cursor:pointer;"
            },
            onclick: handler,
            "{label}"
        }
    }
}

fn workspace_button<F>(on: bool, label: &'static str, onclick: F) -> Element
where
    F: Fn(MouseEvent) + Clone + 'static,
{
    let handler = onclick.clone();
    rsx! {
        button {
            style: if on {
                "padding:10px 14px; border:none; border-radius:12px; background:#1f4d3b; color:white; font-weight:700; cursor:pointer;"
            } else {
                "padding:10px 14px; border:1px solid #d7d2c6; border-radius:12px; background:white; color:#1f4d3b; font-weight:700; cursor:pointer;"
            },
            onclick: handler,
            "{label}"
        }
    }
}

fn subtab_button<F>(on: bool, label: impl Into<String>, onclick: F) -> Element
where
    F: Fn(MouseEvent) + Clone + 'static,
{
    let handler = onclick.clone();
    let label = label.into();
    rsx! {
        button {
            style: if on {
                "padding:8px 12px; border:none; border-radius:10px; background:#d96f32; color:white; font-weight:700; cursor:pointer;"
            } else {
                "padding:8px 12px; border:1px solid #d7d2c6; border-radius:10px; background:white; color:#6a5f4e; font-weight:700; cursor:pointer;"
            },
            onclick: handler,
            "{label}"
        }
    }
}

fn empty_block(message: &'static str) -> Element {
    rsx! {
        div {
            style: "padding:24px; border-radius:16px; background:white; border:1px dashed #d7d2c6; color:#7a7366; text-align:center;",
            "{message}"
        }
    }
}

fn comic_row(comic: hmanga_core::Comic, on_pick: Rc<dyn Fn(String) + 'static>) -> Element {
    let comic_id = comic.id.clone();
    let on_pick_click = on_pick.clone();
    rsx! {
        div {
            key: "{comic.id}",
            style: "padding:14px 16px; border-radius:14px; background:white; border:1px solid #ebe4d8; margin-bottom:10px; box-shadow:0 8px 24px rgba(60,40,10,0.04);",
            div { style: "font-weight:800; line-height:1.4;", "{comic.title}" }
            div { style: "margin-top:8px; font-size:13px; color:#7a7366;", "作者：{comic.author}" }
            div { style: "margin-top:8px; display:flex; gap:6px; flex-wrap:wrap;",
                for tag in comic.tags {
                    span {
                        style: "padding:4px 8px; border-radius:999px; background:#f0eadc; color:#5f5648; font-size:12px;",
                        "{tag}"
                    }
                }
            }
            button {
                style: "margin-top:12px; padding:8px 12px; border:none; border-radius:10px; background:#1f4d3b; color:white; font-weight:700; cursor:pointer;",
                onclick: move |_| on_pick_click(comic_id.clone()),
                "查看章节"
            }
        }
    }
}

fn local_comic_card(
    item: crate::service::LocalComicEntry,
    on_open: Rc<dyn Fn(LocalChapterEntry) + 'static>,
    on_export_cbz: Rc<dyn Fn(LocalChapterEntry) + 'static>,
    on_delete: Rc<dyn Fn(std::path::PathBuf) + 'static>,
) -> Element {
    let comic_dir = item.comic_dir.clone();
    let delete_handler = on_delete.clone();
    rsx! {
        div {
            key: "{item.comic.id}",
            style: "padding:14px 16px; border-radius:16px; background:white; border:1px solid #ebe4d8; margin-bottom:12px;",
            div { style: "display:flex; align-items:center; gap:8px;",
                div { style: "flex:1;",
                    div { style: "font-weight:800;", "{item.comic.title}" }
                    div { style: "display:flex; align-items:center; gap:8px; font-size:12px; color:#7a7366;",
                        span { "{item.chapters.len()} 个已下载章节" }
                        if let Some(platform_tag) = item.platform_tag.clone() {
                            span {
                                style: "padding:3px 7px; border-radius:999px; background:#efe4cf; color:#7b5e2d; font-weight:700;",
                                "{platform_tag}"
                            }
                        }
                    }
                }
                button {
                    style: "padding:8px 10px; border:none; border-radius:10px; background:#b23b2c; color:white; font-weight:700; cursor:pointer;",
                    onclick: move |_| delete_handler(comic_dir.clone()),
                    "删除"
                }
            }
            div { style: "margin-top:12px; display:flex; flex-direction:column; gap:8px;",
                for chapter in item.chapters {
                    {
                        let export_chapter = chapter.clone();
                        let open_chapter = chapter.clone();
                        rsx! {
                            div {
                                key: "{chapter.chapter_id}",
                                style: "display:flex; align-items:center; gap:8px; padding:10px 12px; border-radius:12px; background:#faf7f0;",
                                div { style: "flex:1; font-size:13px;", "{chapter.chapter_title}" }
                                button {
                                    style: "padding:7px 10px; border:none; border-radius:10px; background:#8a6f2f; color:white; font-weight:700; cursor:pointer;",
                                    onclick: {
                                        let export_handler = on_export_cbz.clone();
                                        move |_| export_handler(export_chapter.clone())
                                    },
                                    "导出 CBZ"
                                }
                                button {
                                    style: "padding:7px 10px; border:none; border-radius:10px; background:#d96f32; color:white; font-weight:700; cursor:pointer;",
                                    onclick: {
                                        let open_handler = on_open.clone();
                                        move |_| open_handler(open_chapter.clone())
                                    },
                                    "阅读"
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

fn download_chapter_row(
    comic: hmanga_core::Comic,
    chapter: hmanga_core::ChapterInfo,
    library: Vec<crate::service::LocalComicEntry>,
    services: AppServices,
    mut ui: Signal<UiState>,
) -> Element {
    let chapter_dir = preview_chapter_dir(&services, &comic.title, &chapter.title);
    let local_chapter = find_local_chapter(&library, &chapter.id).or_else(|| {
        partial_chapter_from_download(&DownloadRow {
            chapter_id: chapter.id.clone(),
            label: format!("{} / {}", comic.title, chapter.title),
            comic_title: comic.title.clone(),
            chapter_title: chapter.title.clone(),
            chapter_dir,
            status: DownloadRowState::Downloading,
            detail: String::new(),
            downloaded_pages: 0,
            total_pages: 0,
            current_item: String::new(),
        })
    });
    rsx! {
        div {
            key: "{chapter.id}",
            style: "display:flex; align-items:center; gap:12px; padding:14px 16px; border-radius:14px; background:white; border:1px solid #ebe4d8;",
            div { style: "flex:1; min-width:0;",
                div { style: "font-weight:700;", "{chapter.title}" }
                div { style: "font-size:12px; color:#7a7366;", "章节 ID: {chapter.id}" }
            }
            if let Some(local_chapter) = local_chapter.clone() {
                button {
                    style: "padding:8px 12px; border:none; border-radius:10px; background:#1f4d3b; color:white; font-weight:700; cursor:pointer;",
                    onclick: move |_| {
                        ui.with_mut(|state| {
                            state.open_reader(ReaderState {
                                title: format!("{} / {}", local_chapter.comic_title, local_chapter.chapter_title),
                                pages: local_chapter.pages.iter().map(|path| to_browser_src(path)).collect(),
                                current_index: 0,
                                source_dir: Some(local_chapter.chapter_dir.clone()),
                            });
                            state.status = format!("打开阅读器：{}", state.reader.title);
                        });
                    },
                    "阅读"
                }
            }
            button {
                style: button_style(true),
                onclick: move |_| {
                    let mut ui_handle = ui;
                    let services_handle = services.clone();
                    let comic = comic.clone();
                    let chapter = chapter.clone();
                    ui_handle.with_mut(|state| {
                        state.downloads.insert(0, DownloadRow {
                            chapter_id: chapter.id.clone(),
                            label: format!("{} / {}", comic.title, chapter.title),
                            comic_title: comic.title.clone(),
                            chapter_title: chapter.title.clone(),
                            chapter_dir: preview_chapter_dir(&services_handle, &comic.title, &chapter.title),
                            status: DownloadRowState::Downloading,
                            detail: "任务已创建".to_string(),
                            downloaded_pages: 0,
                            total_pages: 0,
                            current_item: String::new(),
                        });
                    });
                    let services_task = services_handle.clone();
                    spawn(async move {
                        let result = services_task
                            .download_jm_chapter(&comic, &chapter, {
                                let mut ui_progress = ui_handle;
                                let chapter_id = chapter.id.clone();
                                move |downloaded, total, current_item| {
                                    ui_progress.with_mut(|state| {
                                        if let Some(row) = state
                                            .downloads
                                            .iter_mut()
                                            .find(|row| row.chapter_id == chapter_id)
                                        {
                                            row.downloaded_pages = downloaded;
                                            row.total_pages = total;
                                            row.current_item = current_item.to_string();
                                            row.detail = if total == 0 {
                                                "准备下载资源...".to_string()
                                            } else if downloaded >= total {
                                                format!("已完成 {downloaded}/{total}")
                                            } else {
                                                format!("已下载 {downloaded}/{total} · {current_item}")
                                            };
                                        }
                                        refresh_reader_from_download(state, &chapter_id);
                                    });
                                }
                            })
                            .await;
                        let library = services_task.read_library().unwrap_or_default();
                        ui_handle.with_mut(|state| {
                            if let Some(row) = state.downloads.iter_mut().find(|row| row.chapter_id == chapter.id) {
                                row.status = match result {
                                    Ok(_) => DownloadRowState::Completed,
                                    Err(ref err) if err == "下载已取消" => DownloadRowState::Cancelled,
                                    Err(_) => DownloadRowState::Failed,
                                };
                                row.detail = match &result {
                                    Ok(_) => "下载完成".to_string(),
                                    Err(err) => err.clone(),
                                };
                                if matches!(row.status, DownloadRowState::Completed) && row.total_pages == 0 {
                                    row.total_pages = row.downloaded_pages;
                                }
                            }
                            state.library = library;
                            state.status = match result {
                                Ok(local_chapter) => {
                                    state.open_reader(ReaderState {
                                        title: format!("{} / {}", local_chapter.comic_title, local_chapter.chapter_title),
                                        pages: local_chapter.pages.iter().map(|path| to_browser_src(path)).collect(),
                                        current_index: 0,
                                        source_dir: Some(local_chapter.chapter_dir.clone()),
                                    });
                                    format!("{} 下载完成。", chapter.title)
                                }
                                Err(err) => err,
                            };
                        });
                    });
                },
                "下载"
            }
        }
    }
}

fn download_queue_row(
    row: DownloadRow,
    library: Vec<crate::service::LocalComicEntry>,
    services: AppServices,
    mut ui: Signal<UiState>,
) -> Element {
    let local_chapter = find_local_chapter(&library, &row.chapter_id)
        .or_else(|| partial_chapter_from_download(&row));
    let pause_services = services.clone();
    let cancel_services = services.clone();
    let resume_services = services.clone();
    let cancel_paused_services = services.clone();
    let pause_id = row.chapter_id.clone();
    let cancel_id = row.chapter_id.clone();
    let resume_id = row.chapter_id.clone();
    let cancel_paused_id = row.chapter_id.clone();
    let cleanup_id = row.chapter_id.clone();
    rsx! {
        div {
            key: "{row.chapter_id}",
            style: "padding:14px 16px; border-radius:14px; background:white; border:1px solid #ebe4d8; margin-bottom:10px;",
            div { style: "font-weight:700;", "{row.label}" }
            div { style: "margin-top:6px; color:#7a7366;", "{row.status.label()}" }
            if !row.detail.is_empty() {
                div { style: "margin-top:4px; color:#9a9385; font-size:12px;", "{row.detail}" }
            }
            if row.total_pages > 0 {
                div {
                    style: "margin-top:4px; color:#9a9385; font-size:12px;",
                    "进度 {row.downloaded_pages}/{row.total_pages}"
                    if !row.current_item.is_empty() {
                        span { " · 当前 {row.current_item}" }
                    }
                }
            }
            div { style: "display:flex; gap:8px; flex-wrap:wrap; margin-top:10px;",
                if matches!(row.status, DownloadRowState::Downloading) {
                    button {
                        style: "padding:8px 10px; border:none; border-radius:10px; background:#8a6f2f; color:white; font-weight:700; cursor:pointer;",
                        onclick: move |_| {
                            services_pause(&pause_services, &pause_id, &mut ui);
                        },
                        "暂停"
                    }
                    button {
                        style: "padding:8px 10px; border:none; border-radius:10px; background:#b23b2c; color:white; font-weight:700; cursor:pointer;",
                        onclick: move |_| {
                            services_cancel(&cancel_services, &cancel_id, &mut ui);
                        },
                        "取消"
                    }
                }
                if matches!(row.status, DownloadRowState::Paused) {
                    button {
                        style: "padding:8px 10px; border:none; border-radius:10px; background:#1f4d3b; color:white; font-weight:700; cursor:pointer;",
                        onclick: move |_| {
                            services_resume(&resume_services, &resume_id, &mut ui);
                        },
                        "继续"
                    }
                    button {
                        style: "padding:8px 10px; border:none; border-radius:10px; background:#b23b2c; color:white; font-weight:700; cursor:pointer;",
                        onclick: move |_| {
                            services_cancel(&cancel_paused_services, &cancel_paused_id, &mut ui);
                        },
                        "取消"
                    }
                }
                if matches!(row.status, DownloadRowState::Completed | DownloadRowState::Failed | DownloadRowState::Cancelled) {
                    button {
                        style: "padding:8px 10px; border:none; border-radius:10px; background:#ddd4c6; color:#4f473a; font-weight:700; cursor:pointer;",
                        onclick: move |_| {
                            ui.with_mut(|state| state.downloads.retain(|download| download.chapter_id != cleanup_id));
                        },
                        "清理"
                    }
                }
            }
            if let Some(local_chapter) = local_chapter {
                button {
                    style: "margin-top:10px; padding:8px 12px; border:none; border-radius:10px; background:#1f4d3b; color:white; font-weight:700; cursor:pointer;",
                    onclick: move |_| {
                        ui.with_mut(|state| {
                            state.open_reader(ReaderState {
                                title: format!("{} / {}", local_chapter.comic_title, local_chapter.chapter_title),
                                pages: local_chapter.pages.iter().map(|path| to_browser_src(path)).collect(),
                                current_index: 0,
                                source_dir: Some(local_chapter.chapter_dir.clone()),
                            });
                            state.status = format!("打开阅读器：{}", state.reader.title);
                        });
                    },
                    "阅读"
                }
            }
        }
    }
}

fn enqueue_download(
    ui_handle: &mut Signal<UiState>,
    services: AppServices,
    comic: hmanga_core::Comic,
    chapter: hmanga_core::ChapterInfo,
) {
    ui_handle.with_mut(|state| {
        if state
            .downloads
            .iter()
            .any(|row| row.chapter_id == chapter.id)
        {
            return;
        }
        state.downloads.insert(
            0,
            DownloadRow {
                chapter_id: chapter.id.clone(),
                label: format!("{} / {}", comic.title, chapter.title),
                comic_title: comic.title.clone(),
                chapter_title: chapter.title.clone(),
                chapter_dir: preview_chapter_dir(&services, &comic.title, &chapter.title),
                status: DownloadRowState::Downloading,
                detail: "任务已创建".to_string(),
                downloaded_pages: 0,
                total_pages: 0,
                current_item: String::new(),
            },
        );
    });

    let mut ui_task = *ui_handle;
    spawn(async move {
        let result = services
            .download_jm_chapter(&comic, &chapter, {
                let mut ui_progress = ui_task;
                let chapter_id = chapter.id.clone();
                move |downloaded, total, current_item| {
                    ui_progress.with_mut(|state| {
                        if let Some(row) = state
                            .downloads
                            .iter_mut()
                            .find(|row| row.chapter_id == chapter_id)
                        {
                            row.downloaded_pages = downloaded;
                            row.total_pages = total;
                            row.current_item = current_item.to_string();
                            row.detail = if total == 0 {
                                "准备下载资源...".to_string()
                            } else if downloaded >= total {
                                format!("已完成 {downloaded}/{total}")
                            } else {
                                format!("已下载 {downloaded}/{total} · {current_item}")
                            };
                        }
                        refresh_reader_from_download(state, &chapter_id);
                    });
                }
            })
            .await;
        let library = services.read_library().unwrap_or_default();
        ui_task.with_mut(|state| {
            if let Some(row) = state
                .downloads
                .iter_mut()
                .find(|row| row.chapter_id == chapter.id)
            {
                row.status = match result {
                    Ok(_) => DownloadRowState::Completed,
                    Err(ref err) if err == "下载已取消" => DownloadRowState::Cancelled,
                    Err(_) => DownloadRowState::Failed,
                };
                row.detail = match &result {
                    Ok(_) => "下载完成".to_string(),
                    Err(err) => err.clone(),
                };
                if matches!(row.status, DownloadRowState::Completed) && row.total_pages == 0 {
                    row.total_pages = row.downloaded_pages;
                }
            }
            state.library = library;
        });
    });
}

fn services_pause(services: &AppServices, chapter_id: &str, ui: &mut Signal<UiState>) {
    services.pause_download(chapter_id);
    ui.with_mut(|state| {
        if let Some(row) = state
            .downloads
            .iter_mut()
            .find(|row| row.chapter_id == chapter_id)
        {
            row.status = DownloadRowState::Paused;
            row.detail = "已请求暂停".to_string();
        }
    });
}

fn services_resume(services: &AppServices, chapter_id: &str, ui: &mut Signal<UiState>) {
    services.resume_download(chapter_id);
    ui.with_mut(|state| {
        if let Some(row) = state
            .downloads
            .iter_mut()
            .find(|row| row.chapter_id == chapter_id)
        {
            row.status = DownloadRowState::Downloading;
            row.detail = "继续下载".to_string();
        }
    });
}

fn services_cancel(services: &AppServices, chapter_id: &str, ui: &mut Signal<UiState>) {
    services.cancel_download(chapter_id);
    ui.with_mut(|state| {
        if let Some(row) = state
            .downloads
            .iter_mut()
            .find(|row| row.chapter_id == chapter_id)
        {
            row.status = DownloadRowState::Cancelled;
            row.detail = "已请求取消".to_string();
        }
    });
}

fn reader_panel(
    reader: ReaderState,
    mut ui: Signal<UiState>,
    empty_message: &'static str,
    allow_fullscreen: bool,
) -> Element {
    if reader.pages.is_empty() {
        return rsx! {{empty_block(empty_message)}};
    }

    rsx! {
        div {
            style: "display:flex; flex-direction:column; gap:12px;",
            tabindex: 0,
            onkeydown: move |evt| {
                match evt.key() {
                    Key::ArrowLeft | Key::ArrowUp => {
                        ui.with_mut(|state| {
                            if state.reader.current_index > 0 {
                                state.reader.current_index -= 1;
                            }
                        });
                    }
                    Key::ArrowRight | Key::ArrowDown => {
                        ui.with_mut(|state| {
                            if state.reader.current_index + 1 < state.reader.pages.len() {
                                state.reader.current_index += 1;
                            }
                        });
                    }
                    Key::Home => {
                        ui.with_mut(|state| {
                            state.reader.current_index = 0;
                        });
                    }
                    Key::End => {
                        ui.with_mut(|state| {
                            state.reader.current_index = state.reader.pages.len().saturating_sub(1);
                        });
                    }
                    _ => {}
                }
            },
            div { style: "display:flex; align-items:center; gap:8px;",
                button {
                    style: button_style(reader.current_index > 0),
                    disabled: reader.current_index == 0,
                    onclick: move |_| ui.with_mut(|state| {
                        if state.reader.current_index > 0 {
                            state.reader.current_index -= 1;
                        }
                    }),
                    "上一页"
                }
                div { style: "flex:1; text-align:center; font-weight:700;", "{reader.title}" }
                if allow_fullscreen {
                    button {
                        style: "padding:8px 12px; border:none; border-radius:10px; background:#1f4d3b; color:white; font-weight:700; cursor:pointer;",
                        onclick: move |_| ui.with_mut(|state| state.open_reader_fullscreen()),
                        "最大化"
                    }
                }
                button {
                    style: button_style(reader.current_index + 1 < reader.pages.len()),
                    disabled: reader.current_index + 1 >= reader.pages.len(),
                    onclick: move |_| ui.with_mut(|state| {
                        if state.reader.current_index + 1 < state.reader.pages.len() {
                            state.reader.current_index += 1;
                        }
                    }),
                    "下一页"
                }
            }
            div { style: "text-align:center; color:#7a7366;", "第 {reader.current_index + 1} / {reader.pages.len()} 页" }
            img {
                style: "width:100%; max-height:70vh; object-fit:contain; border-radius:16px; border:1px solid #ebe4d8; background:white;",
                src: "{reader.pages[reader.current_index]}"
            }
        }
    }
}

fn find_local_chapter(
    library: &[crate::service::LocalComicEntry],
    chapter_id: &str,
) -> Option<LocalChapterEntry> {
    library
        .iter()
        .flat_map(|comic| comic.chapters.iter())
        .find(|chapter| chapter.chapter_id == chapter_id)
        .cloned()
}

fn refresh_reader_from_download(state: &mut UiState, chapter_id: &str) {
    let Some(source_dir) = state.reader.source_dir.clone() else {
        return;
    };
    let matching_download = state
        .downloads
        .iter()
        .find(|row| row.chapter_id == chapter_id && row.chapter_dir == source_dir);
    let Some(download) = matching_download else {
        return;
    };

    let Some(partial) = partial_chapter_from_download(download) else {
        return;
    };

    let previous_len = state.reader.pages.len();
    state.reader.pages = partial
        .pages
        .iter()
        .map(|path| to_browser_src(path))
        .collect::<Vec<_>>();
    if state.reader.pages.is_empty() {
        state.reader.current_index = 0;
    } else if state.reader.current_index >= state.reader.pages.len() {
        state.reader.current_index = state.reader.pages.len() - 1;
    } else if state.reader.current_index + 1 == previous_len {
        state.reader.current_index = previous_len.saturating_sub(1);
    }
}

fn partial_chapter_from_download(row: &DownloadRow) -> Option<LocalChapterEntry> {
    if !row.chapter_dir.exists() {
        return None;
    }

    let mut pages = std::fs::read_dir(&row.chapter_dir)
        .ok()?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.extension()
                .and_then(|ext| ext.to_str())
                .map(|ext| matches!(ext, "png" | "gif" | "jpg" | "jpeg" | "webp"))
                .unwrap_or(false)
        })
        .collect::<Vec<_>>();
    pages.sort();
    if pages.is_empty() {
        return None;
    }

    Some(LocalChapterEntry {
        comic_id: String::new(),
        comic_title: row.comic_title.clone(),
        chapter_id: row.chapter_id.clone(),
        chapter_title: row.chapter_title.clone(),
        chapter_dir: row.chapter_dir.clone(),
        pages,
    })
}

fn preview_chapter_dir(
    services: &AppServices,
    comic_title: &str,
    chapter_title: &str,
) -> std::path::PathBuf {
    let config = services.config();
    config
        .download_dir
        .join(ui_filename_filter(comic_title))
        .join(ui_filename_filter(chapter_title))
}

fn ui_filename_filter(value: &str) -> String {
    value
        .chars()
        .map(|ch| match ch {
            '\\' | '/' | '\n' => ' ',
            ':' => '：',
            '*' => '⭐',
            '?' => '？',
            '"' => '\'',
            '<' => '《',
            '>' => '》',
            '|' => '丨',
            _ => ch,
        })
        .collect::<String>()
        .trim()
        .trim_end_matches('.')
        .trim()
        .to_string()
}
