use std::cmp::PartialEq;

#[derive(Debug, Clone, PartialEq)]
pub enum Route {
    Home,
    Search,
    Downloads,
    Settings,
}

impl Route {
    pub fn label(&self) -> &'static str {
        match self {
            Route::Home => "首页",
            Route::Search => "搜索",
            Route::Downloads => "下载",
            Route::Settings => "设置",
        }
    }
}
