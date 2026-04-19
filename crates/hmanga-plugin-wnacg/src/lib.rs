pub use hmanga_plugin_macro::hmanga_plugin;

use std::collections::HashMap;

use async_trait::async_trait;
use hmanga_core::{
    Capabilities, ChapterInfo, Comic, DynPlugin, FavoriteResult, HostApi, HttpMethod, HttpRequest,
    ImageUrl, PluginError, PluginMetaInfo, PluginResult, SearchResult, SearchSort, Session,
    WeeklyResult,
};
use regex::Regex;
use scraper::{Html, Selector};
use serde::Deserialize;

const DEFAULT_API_DOMAIN: &str = "www.wnacg.com";

#[derive(Debug, Clone)]
pub struct WnacgPlugin {
    api_domain: String,
    download_format: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WnacgUserProfile {
    pub username: String,
    pub email: String,
    pub favorites_count: i64,
    pub favorites_max: i64,
}

impl Default for WnacgPlugin {
    fn default() -> Self {
        Self {
            api_domain: DEFAULT_API_DOMAIN.to_string(),
            download_format: "webp".to_string(),
        }
    }
}

impl WnacgPlugin {
    pub fn with_api_domain(mut self, api_domain: impl Into<String>) -> Self {
        self.api_domain = api_domain.into();
        self
    }

    pub fn with_download_format(mut self, format: impl Into<String>) -> Self {
        self.download_format = format.into();
        self
    }

    pub fn meta(&self) -> PluginMetaInfo {
        PluginMetaInfo {
            id: plugin_id().to_string(),
            name: "绅士漫画".to_string(),
            version: "0.1.0".to_string(),
            sdk_version: 1,
            icon: Vec::new(),
            description: "wnacg.com site adapter".to_string(),
            capabilities: Capabilities {
                search: true,
                login: true,
                favorites: true,
                ranking: false,
                weekly: false,
                tags_browsing: true,
            },
        }
    }

    pub async fn login(
        &self,
        host: &dyn HostApi,
        username: &str,
        password: &str,
    ) -> Result<Session, PluginError> {
        let form = form_encode(&[("login_name", username), ("login_pass", password)]);

        let request = HttpRequest {
            url: format!("https://{}/users-check_login.html", self.api_domain),
            method: HttpMethod::Post,
            headers: HashMap::from([
                (
                    "referer".to_string(),
                    format!("https://{}/", self.api_domain),
                ),
                (
                    "content-type".to_string(),
                    "application/x-www-form-urlencoded".to_string(),
                ),
            ]),
            body: Some(form.into_bytes()),
        };

        let response = host.http_request(request).await?;

        if response.status != 200 {
            return Err(PluginError::Network(format!(
                "unexpected status: {}",
                response.status
            )));
        }

        let cookie = response
            .header_values("set-cookie")
            .and_then(cookie_header_from_set_cookie)
            .ok_or_else(|| PluginError::Parse("missing set-cookie header".to_string()))?;
        let body =
            String::from_utf8(response.body).map_err(|err| PluginError::Parse(err.to_string()))?;

        let login_resp: LoginResp =
            serde_json::from_str(&body).map_err(|err| PluginError::Parse(err.to_string()))?;

        if !login_resp.ret {
            return Err(PluginError::Auth("login failed".to_string()));
        }

        Ok(Session {
            token: cookie.clone(),
            username: username.to_string(),
            extra: HashMap::from([("cookie".to_string(), cookie)]),
        })
    }

    pub async fn get_user_profile(
        &self,
        host: &dyn HostApi,
        session: &Session,
    ) -> Result<WnacgUserProfile, PluginError> {
        let cookie = session
            .extra
            .get("cookie")
            .ok_or_else(|| PluginError::Auth("missing cookie".to_string()))?;

        let request = HttpRequest {
            url: format!("https://{}/users.html", self.api_domain),
            method: HttpMethod::Get,
            headers: HashMap::from([
                ("cookie".to_string(), cookie.clone()),
                (
                    "referer".to_string(),
                    format!("https://{}/", self.api_domain),
                ),
            ]),
            body: None,
        };

        let response = host.http_request(request).await?;
        let body =
            String::from_utf8(response.body).map_err(|err| PluginError::Parse(err.to_string()))?;

        self.parse_user_profile(&body)
    }

    pub async fn search(
        &self,
        host: &dyn HostApi,
        keyword: &str,
        page: u32,
        _sort: SearchSort,
    ) -> Result<SearchResult, PluginError> {
        let request = HttpRequest {
            url: format!(
                "https://{}/search/index.php?q={}&syn=yes&f=_all&s=create_time_DESC&p={}",
                self.api_domain,
                percent_encode(keyword),
                page
            ),
            method: HttpMethod::Get,
            headers: HashMap::from([(
                "referer".to_string(),
                format!("https://{}/", self.api_domain),
            )]),
            body: None,
        };

        let response = host.http_request(request).await?;
        let body =
            String::from_utf8(response.body).map_err(|err| PluginError::Parse(err.to_string()))?;

        self.parse_search_results(&body, page)
    }

    pub async fn search_by_tag(
        &self,
        host: &dyn HostApi,
        tag: &str,
        page: u32,
    ) -> Result<SearchResult, PluginError> {
        let request = HttpRequest {
            url: format!(
                "https://{}/albums-index-page-{}-tag-{}.html",
                self.api_domain, page, tag
            ),
            method: HttpMethod::Get,
            headers: HashMap::from([(
                "referer".to_string(),
                format!("https://{}/", self.api_domain),
            )]),
            body: None,
        };

        let response = host.http_request(request).await?;
        let body =
            String::from_utf8(response.body).map_err(|err| PluginError::Parse(err.to_string()))?;

        self.parse_search_results(&body, page)
    }

    pub async fn get_comic(
        &self,
        host: &dyn HostApi,
        comic_id: &str,
    ) -> Result<Comic, PluginError> {
        let request = HttpRequest {
            url: format!(
                "https://{}/photos-index-aid-{}.html",
                self.api_domain, comic_id
            ),
            method: HttpMethod::Get,
            headers: HashMap::from([(
                "referer".to_string(),
                format!("https://{}/", self.api_domain),
            )]),
            body: None,
        };

        let response = host.http_request(request).await?;
        let body =
            String::from_utf8(response.body).map_err(|err| PluginError::Parse(err.to_string()))?;

        let img_list = self.get_img_list(host, comic_id).await?;

        self.parse_comic(&body, comic_id, img_list)
    }

    pub async fn get_chapter_images(
        &self,
        host: &dyn HostApi,
        chapter_id: &str,
    ) -> Result<Vec<ImageUrl>, PluginError> {
        let urls = self.get_img_list(host, chapter_id).await?;
        Ok(urls
            .into_iter()
            .enumerate()
            .map(|(index, url)| ImageUrl {
                url,
                headers: HashMap::new(),
                index: index as u32,
            })
            .collect())
    }

    pub async fn get_favorites(
        &self,
        host: &dyn HostApi,
        session: &Session,
        folder_id: i64,
        page: u32,
    ) -> Result<SearchResult, PluginError> {
        let cookie = session
            .extra
            .get("cookie")
            .ok_or_else(|| PluginError::Auth("missing cookie".to_string()))?;

        let request = HttpRequest {
            url: format!(
                "https://{}/users-users_fav-page-{}-c-{}.html",
                self.api_domain, page, folder_id
            ),
            method: HttpMethod::Get,
            headers: HashMap::from([
                ("cookie".to_string(), cookie.clone()),
                (
                    "referer".to_string(),
                    format!("https://{}/", self.api_domain),
                ),
            ]),
            body: None,
        };

        let response = host.http_request(request).await?;
        let body =
            String::from_utf8(response.body).map_err(|err| PluginError::Parse(err.to_string()))?;

        self.parse_search_results(&body, page)
    }

    async fn get_img_list(
        &self,
        host: &dyn HostApi,
        comic_id: &str,
    ) -> Result<Vec<String>, PluginError> {
        let request = HttpRequest {
            url: format!(
                "https://{}/photos-gallery-aid-{}.html",
                self.api_domain, comic_id
            ),
            method: HttpMethod::Get,
            headers: HashMap::from([(
                "referer".to_string(),
                format!("https://{}/", self.api_domain),
            )]),
            body: None,
        };

        let response = host.http_request(request).await?;
        let body =
            String::from_utf8(response.body).map_err(|err| PluginError::Parse(err.to_string()))?;

        self.parse_img_list(&body)
    }

    fn parse_user_profile(&self, html: &str) -> Result<WnacgUserProfile, PluginError> {
        let document = Html::parse_document(html);

        let username_selector = Selector::parse(".user_name").unwrap();
        let email_selector = Selector::parse(".user_email").unwrap();

        let username = document
            .select(&username_selector)
            .next()
            .map(|el| el.text().collect::<String>())
            .unwrap_or_default();

        let email = document
            .select(&email_selector)
            .next()
            .map(|el| el.text().collect::<String>())
            .unwrap_or_default();

        Ok(WnacgUserProfile {
            username,
            email,
            favorites_count: 0,
            favorites_max: 0,
        })
    }

    fn parse_search_results(&self, html: &str, page: u32) -> Result<SearchResult, PluginError> {
        if is_cloudflare_challenge_page(html) {
            return Err(PluginError::Network(
                "Cloudflare challenge blocked WNACG search; a JS-capable browser session is required"
                    .to_string(),
            ));
        }

        let document = Html::parse_document(html);

        let comic_selector =
            Selector::parse(
                ".li.gallary_item, li.gallary_item, .pic_box, .comic-index-item, .thumb-item, .col-sm-6",
            )
            .unwrap();
        let title_link_selector = Selector::parse(".title > a, a[title]").unwrap();
        let cover_selector = Selector::parse("img[src]").unwrap();
        let total_selector = Selector::parse("#bodywrap .result > b").unwrap();

        let mut comics = Vec::new();
        for item in document.select(&comic_selector) {
            let Some(title_link) = item.select(&title_link_selector).next() else {
                continue;
            };
            let title = title_link
                .value()
                .attr("title")
                .map(ToOwned::to_owned)
                .unwrap_or_else(|| title_link.text().collect::<String>().trim().to_string());

            let cover_url = item
                .select(&cover_selector)
                .next()
                .and_then(|el| el.value().attr("src"))
                .map(|s| {
                    if s.starts_with("http://") || s.starts_with("https://") {
                        s.to_string()
                    } else if s.starts_with("//") {
                        format!("https:{}", s)
                    } else if s.starts_with('/') {
                        format!("https://{}{}", self.api_domain, s)
                    } else {
                        format!("https://{}/{}", self.api_domain, s)
                    }
                })
                .unwrap_or_default();

            let detail_url = title_link.value().attr("href").unwrap_or_default();

            let comic_id = extract_comic_id_from_url(detail_url);

            if comic_id.is_empty() {
                continue;
            }

            comics.push(Comic {
                id: comic_id,
                source: plugin_id().to_string(),
                title,
                author: String::new(),
                cover_url,
                description: String::new(),
                tags: Vec::new(),
                chapters: Vec::new(),
                extra: HashMap::new(),
                ..Default::default()
            });
        }

        let total_pages = document
            .select(&total_selector)
            .next()
            .map(|el| el.text().collect::<String>())
            .map(|text| text.replace(',', ""))
            .and_then(|text| text.trim().parse::<u32>().ok())
            .map(|total| total.div_ceil(24).max(page))
            .unwrap_or_else(|| {
                if comics.is_empty() && page == 1 {
                    1
                } else {
                    page
                }
            });

        Ok(SearchResult {
            comics,
            current_page: page,
            total_pages,
        })
    }

    fn parse_comic(
        &self,
        html: &str,
        comic_id: &str,
        img_list: Vec<String>,
    ) -> Result<Comic, PluginError> {
        let document = Html::parse_document(html);

        let title_selector = Selector::parse(".info_tag h3, .comic-title, h3.title").unwrap();
        let cover_selector = Selector::parse(".cover img, .comic-cover img").unwrap();
        let desc_selector = Selector::parse(".intro, .comic-description, .info_intro").unwrap();
        let tag_selector = Selector::parse(".tag a, .tags a, .info_tag a").unwrap();

        let title = document
            .select(&title_selector)
            .next()
            .map(|el| el.text().collect::<String>())
            .unwrap_or_default();

        let cover_url = document
            .select(&cover_selector)
            .next()
            .and_then(|el| el.value().attr("src"))
            .map(|s| {
                if s.starts_with("http://") || s.starts_with("https://") {
                    s.to_string()
                } else if s.starts_with("//") {
                    format!("https:{}", s)
                } else if s.starts_with('/') {
                    format!("https://{}{}", self.api_domain, s)
                } else {
                    format!("https://{}/{}", self.api_domain, s)
                }
            })
            .unwrap_or_default();

        let description = document
            .select(&desc_selector)
            .next()
            .map(|el| el.text().collect::<String>())
            .unwrap_or_default();

        let tags = document
            .select(&tag_selector)
            .filter_map(|el| el.text().collect::<String>().into())
            .collect::<Vec<_>>();

        let chapters = vec![ChapterInfo {
            id: comic_id.to_string(),
            title: "全部图片".to_string(),
            page_count: Some(img_list.len() as u32),
        }];

        Ok(Comic {
            id: comic_id.to_string(),
            source: plugin_id().to_string(),
            title,
            author: String::new(),
            cover_url,
            description,
            tags,
            chapters,
            extra: HashMap::new(),
            ..Default::default()
        })
    }

    fn parse_img_list(&self, html: &str) -> Result<Vec<String>, PluginError> {
        let re = Regex::new(r#"var\s+imglist\s*=\s*\[(.*?)\];?"#).unwrap();

        let caps = re
            .captures(html)
            .ok_or_else(|| PluginError::Parse("imglist not found".to_string()))?;

        let json_str = caps
            .get(1)
            .ok_or_else(|| PluginError::Parse("imglist content not found".to_string()))?
            .as_str();

        let cleaned = json_str
            .replace("url:", "\"url\":")
            .replace("caption:", "\"caption\":")
            .replace("fast_img_host+", "")
            .replace("\\\"", "\"")
            .replace("\"+", "")
            .replace("+\"", "");

        let img_list: Vec<ImgItem> = serde_json::from_str(&format!("[{}]", cleaned))
            .map_err(|err| PluginError::Parse(format!("failed to parse imglist: {}", err)))?;

        Ok(img_list.into_iter().map(|item| item.url).collect())
    }
}

#[async_trait]
impl DynPlugin for WnacgPlugin {
    fn id(&self) -> &str {
        plugin_id()
    }

    fn meta(&self) -> PluginMetaInfo {
        self.meta()
    }

    async fn search(
        &self,
        host: &dyn HostApi,
        query: &str,
        page: u32,
        sort: SearchSort,
    ) -> PluginResult<SearchResult> {
        self.search(host, query, page, sort).await
    }

    async fn get_comic(&self, host: &dyn HostApi, comic_id: &str) -> PluginResult<Comic> {
        self.get_comic(host, comic_id).await
    }

    async fn get_chapter_images(
        &self,
        host: &dyn HostApi,
        chapter_id: &str,
    ) -> PluginResult<Vec<ImageUrl>> {
        self.get_chapter_images(host, chapter_id).await
    }

    async fn login(
        &self,
        host: &dyn HostApi,
        username: &str,
        password: &str,
    ) -> PluginResult<Session> {
        self.login(host, username, password).await
    }

    async fn get_favorites(
        &self,
        host: &dyn HostApi,
        session: Option<&Session>,
        page: u32,
    ) -> PluginResult<FavoriteResult> {
        let session = session.ok_or(PluginError::Auth("session required".to_string()))?;
        let result = self.get_favorites(host, session, 0, page).await?;
        Ok(FavoriteResult {
            comics: result.comics,
            current_page: result.current_page,
            total_pages: result.total_pages,
            folder_name: None,
        })
    }

    async fn get_weekly(&self, _host: &dyn HostApi) -> PluginResult<WeeklyResult> {
        Err(PluginError::NotSupported)
    }
}

fn extract_comic_id_from_url(url: &str) -> String {
    let re = Regex::new(r"photos-index-aid-(\d+)").unwrap();
    re.captures(url)
        .and_then(|caps| caps.get(1))
        .map(|m| m.as_str().to_string())
        .unwrap_or_default()
}

fn form_encode(entries: &[(&str, &str)]) -> String {
    entries
        .iter()
        .map(|(key, value)| {
            format!(
                "{}={}",
                percent_encode_form(key),
                percent_encode_form(value)
            )
        })
        .collect::<Vec<_>>()
        .join("&")
}

fn percent_encode(value: &str) -> String {
    value
        .bytes()
        .flat_map(|byte| match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                vec![byte as char].into_iter().collect::<Vec<_>>()
            }
            _ => format!("%{:02X}", byte).chars().collect::<Vec<_>>(),
        })
        .collect()
}

fn percent_encode_form(value: &str) -> String {
    value
        .bytes()
        .flat_map(|byte| match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                vec![byte as char]
            }
            b' ' => vec!['+'],
            _ => format!("%{:02X}", byte).chars().collect::<Vec<_>>(),
        })
        .collect()
}

fn cookie_header_from_set_cookie(values: &[String]) -> Option<String> {
    let cookies = values
        .iter()
        .filter_map(|value| value.split(';').next())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .collect::<Vec<_>>();
    if cookies.is_empty() {
        None
    } else {
        Some(cookies.join("; "))
    }
}

fn is_cloudflare_challenge_page(html: &str) -> bool {
    html.contains("Just a moment...")
        || html.contains("Enable JavaScript and cookies to continue")
        || html.contains("_cf_chl_opt")
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct LoginResp {
    ret: bool,
    html: String,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct ImgItem {
    url: String,
    caption: String,
}

pub fn plugin_id() -> &'static str {
    "wnacg"
}
