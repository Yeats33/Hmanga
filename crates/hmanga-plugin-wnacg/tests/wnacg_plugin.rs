use std::collections::{HashMap, VecDeque};
use std::pin::Pin;
use std::sync::{Arc, Mutex};

use hmanga_core::{HostApi, HttpMethod, HttpRequest, HttpResponse, LogLevel};
use hmanga_plugin_wnacg::WnacgPlugin;

const SEARCH_RESULTS_HTML: &str = r#"
<div id="bodywrap">
  <div class="result">共 <b>26</b> 个结果</div>
</div>
<span class="thispage">2</span>
<div class="cc">
  <div class="gallary_wrap">
    <li class="gallary_item">
      <div class="title">
        <a href="/photos-index-aid-123.html" title="作品一">作品一</a>
      </div>
      <img src="//img.wnacg.test/cover-1.jpg" />
      <div class="info_col">10P / today</div>
    </li>
    <li class="gallary_item">
      <div class="title">
        <a href="/photos-index-aid-456.html" title="作品二">作品二</a>
      </div>
      <img src="https://img.wnacg.test/cover-2.jpg" />
      <div class="info_col">12P / yesterday</div>
    </li>
  </div>
</div>
"#;

const CLOUDFLARE_CHALLENGE_HTML: &str = r#"
<!DOCTYPE html>
<html lang="en-US">
  <head>
    <title>Just a moment...</title>
  </head>
  <body>
    <noscript>Enable JavaScript and cookies to continue</noscript>
    <script>window._cf_chl_opt = {};</script>
  </body>
</html>
"#;

#[derive(Clone, Default)]
struct FakeHost {
    responses: Arc<Mutex<VecDeque<HttpResponse>>>,
    requests: Arc<Mutex<Vec<HttpRequest>>>,
}

impl FakeHost {
    fn with_responses(responses: Vec<HttpResponse>) -> Self {
        Self {
            responses: Arc::new(Mutex::new(VecDeque::from(responses))),
            requests: Arc::new(Mutex::new(Vec::new())),
        }
    }

    fn requests(&self) -> Vec<HttpRequest> {
        self.requests.lock().unwrap().clone()
    }
}

impl HostApi for FakeHost {
    fn http_request(
        &self,
        request: HttpRequest,
    ) -> Pin<Box<dyn std::future::Future<Output = hmanga_core::PluginResult<HttpResponse>> + Send>>
    {
        let responses = self.responses.clone();
        let requests = self.requests.clone();
        Box::pin(async move {
            requests.lock().unwrap().push(request);
            responses
                .lock()
                .unwrap()
                .pop_front()
                .ok_or_else(|| hmanga_core::PluginError::Other("missing fake response".to_string()))
        })
    }

    fn log(&self, _level: LogLevel, _msg: &str) {}
}

fn json_response(body: &str, headers: HashMap<String, Vec<String>>) -> HttpResponse {
    HttpResponse {
        status: 200,
        headers,
        body: body.as_bytes().to_vec(),
    }
}

#[tokio::test]
async fn login_posts_form_encoded_credentials() {
    let host = FakeHost::with_responses(vec![json_response(
        r#"{"ret":true,"html":"ok"}"#,
        HashMap::from([(
            "set-cookie".to_string(),
            vec!["session=abc; Path=/; HttpOnly".to_string()],
        )]),
    )]);
    let plugin = WnacgPlugin::default();

    let _ = plugin.login(&host, "demo user", "s3cr?t").await.unwrap();

    let requests = host.requests();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].method, HttpMethod::Post);
    assert_eq!(
        requests[0].headers.get("content-type").map(String::as_str),
        Some("application/x-www-form-urlencoded")
    );
    assert_eq!(
        String::from_utf8_lossy(requests[0].body.as_ref().unwrap()),
        "login_name=demo+user&login_pass=s3cr%3Ft"
    );
}

#[tokio::test]
async fn login_combines_multiple_set_cookie_values_into_cookie_header() {
    let host = FakeHost::with_responses(vec![
        json_response(
            r#"{"ret":true,"html":"ok"}"#,
            HashMap::from([(
                "set-cookie".to_string(),
                vec![
                    "session=abc; Path=/; HttpOnly".to_string(),
                    "member=42; Path=/; Secure".to_string(),
                ],
            )]),
        ),
        json_response(
            r#"<div class="user_name">demo</div><div class="user_email">demo@example.com</div>"#,
            HashMap::new(),
        ),
    ]);
    let plugin = WnacgPlugin::default();

    let session = plugin.login(&host, "demo", "secret").await.unwrap();
    let profile = plugin.get_user_profile(&host, &session).await.unwrap();

    assert_eq!(session.token, "session=abc; member=42");
    assert_eq!(
        session.extra.get("cookie").map(String::as_str),
        Some("session=abc; member=42")
    );
    assert_eq!(profile.username, "demo");

    let requests = host.requests();
    assert_eq!(requests.len(), 2);
    assert_eq!(
        requests[1].headers.get("cookie").map(String::as_str),
        Some("session=abc; member=42")
    );
}

#[tokio::test]
async fn search_parses_gallery_items_from_reference_markup() {
    let host = FakeHost::with_responses(vec![json_response(SEARCH_RESULTS_HTML, HashMap::new())]);
    let plugin = WnacgPlugin::default();

    let result = plugin
        .search(&host, "demo", 2, hmanga_core::SearchSort::Latest)
        .await
        .unwrap();

    assert_eq!(result.current_page, 2);
    assert_eq!(result.total_pages, 2);
    assert_eq!(result.comics.len(), 2);
    assert_eq!(result.comics[0].id, "123");
    assert_eq!(result.comics[0].title, "作品一");
    assert_eq!(
        result.comics[0].cover_url,
        "https://img.wnacg.test/cover-1.jpg"
    );
    assert_eq!(result.comics[1].id, "456");
}

#[tokio::test]
async fn search_returns_error_on_cloudflare_challenge_page() {
    let host = FakeHost::with_responses(vec![json_response(
        CLOUDFLARE_CHALLENGE_HTML,
        HashMap::new(),
    )]);
    let plugin = WnacgPlugin::default();

    let err = plugin
        .search(&host, "demo", 1, hmanga_core::SearchSort::Latest)
        .await
        .unwrap_err();

    assert!(matches!(err, hmanga_core::PluginError::Network(_)));
    assert!(err.to_string().contains("Cloudflare"));
}
