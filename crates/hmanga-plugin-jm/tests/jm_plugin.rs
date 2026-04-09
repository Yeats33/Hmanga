use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};

use hmanga_core::{HostApi, HttpMethod, HttpRequest, HttpResponse, LogLevel, SearchSort};
use hmanga_plugin_jm::JmPlugin;

const SEARCH_RESPONSE: &str = r#"{"code":200,"data":"DEIRZSQxf7DoQq8nDU4/WP1qao+p8d1RKwvCP3wFeqF0DmsqacDrZUhOS1e2iT5zVCN3o5FJPMQzabmY5HAi6m4XIIsOj16+pdSZAO+W0ZWFnsZSyuAKpFM2JJtyZmHASHMZEZ11/7ilv7Yjt1eLwpTYZK7epbw11mJSPJVus6Ukf5xM4OrSvY5f5cn0MGD0c560GAu119o7sLzwgcxwthrIPN+Y2lWWkR2t1at8yKDlGaC3yUMzwiMft7NYw0kyHn4qxo0v1ALDx/vQ6yygl8aqkQDokYq8c5Dov49GCsPcMYdb9mgCCH2Tf7h8Im9hNYOlt5CX12hzbSKBPCnPTw==","errorMsg":""}"#;
const COMIC_RESPONSE: &str = r#"{"code":200,"data":"RuYr/TkbRHi7BiJi/9sDKTpYFnwerF9EtfTP0iP5LX9DKidGp3lbaWjpmYjSXnFeGYR2ejjm1emdMHuoE0Y0EkNabXcmR1Ko7FOmqWmJbcyzCJKhTFP09ChRVfHE+PwAS6DoBeIbEwU18HUtLiQOsDI7334Rv+xXkKxP8VRKSBwbaJJkzkvysj5kfas56RtTMXpsdMUjkmXssDK2lCKhDIGEfIYk8wL3VVM2XSK17ZH5+ud+CktswIMP1kwqSsOnLERy1N7IgLCdmn3H7bG6Um2bhrk8FLnWmS3MqmPfQ62pa70tJYXy/L6hmFJuIybcGPodZa4ZkARAH5EoQU1w49DonbckZBViiDPj912i6mIOdOQ/vmmrFarCez8dk08wSS6YKjfR3ovsxhGORNHhCQR6ypXqlm8ghN7Wg1uJ/e3Cn1WTNxsNJGh9LXyTXGy19ExO4IHSAZilWAmPa+DXywWUdYxL/pPO2Emwoo122suoFLGG6WBQlvxQ+3LczV6jMuZhm0rz1hcxZICHs1H/vk0Rbz0bozkQGUsP7e8jHqs=","errorMsg":""}"#;
const CHAPTER_RESPONSE: &str = r#"{"code":200,"data":"ylwze01QucWpdLZOOMymEoAnhf0Fi4ychIUk842bS1Jw3KvCm9IUTSBl4P9jowHr074JDw47hHuMJM87kOnlO55cITDWAy5v9fKXiWiNJlBw3KvCm9IUTSBl4P9jowHreh7/LrtVI69eurs2eTgJ0+d5sMeBd/nX30frPkJQOMoxG2GwJcQZXY9jF7IStuNlWWaYnc2yx3Isjit+Z8HX63qaHezDW+Hl20DZSNaJYAmyh1Cq+G1ztf//u01F1C3OXuXwoprrF3GEhfv/Srt8xp+XM97EXbQX93hB+MFoPo8=","errorMsg":""}"#;
const PROFILE_RESPONSE: &str = r#"{"code":200,"data":"Jdx/jvA+wlH17Fpezsj2Qhuc+5HjPJFmr1Qwrcc4j5rLc5l3OCNVpn1+WUnHIv5vEcj8thvH1QU//iWMygZ//Wab2Kdj1OQZ7xJqW4GhqmIguux+yCMAeAJxb75wvU5bEZwUTv8H+c5DW8SObxS/vnPE/8+VJuXhSeu6Z6FH4wcsv/eiP65xAM10DJRf4bN64ndsGc7etgdi8jw1HoHSyS/owWX7kZckNCHG/P91WvyrLobiorYNaUpcnUiE91JtYFQszP9q454gvNjNMQIrUvqOWdSFkvfbT2LywuEM1uCNfmsvM4/UWzro2XVz9aWxgWKWQnhD01iDyDeX1yoqx7VPk/5FAss7mQ9JhSIE1BJG0xMyO0KqxuNmEX1Hr5y+d9UfJuyNAhZQ6mK01bq1Pe0vmXzsCYkArQLKIj/LMM5Ooc0q88Af+EjeqSd9JGQVFKROGsAcQgNRT9Q6HSfT9QDgavUBvgqHCSoOWeAI31A9L82ZpH+omDmH77qLWOn+o9HtXgjv+KET9w+UZB+1Ew==","errorMsg":""}"#;
const FAVORITES_RESPONSE: &str = r#"{"code":200,"data":"YRST8A03KqN5/LfljQgfjeeOUWMuEYhax53csTqGbYpPjb+MhyPT8mCFXKk9ltdQpwsqjN2r7js1LKsH/KQ+Z5zbqpmT8JWSU3Otj+50ErsDFY8sxpZiDfMtwqOqhbagTtIJET0gHOAPfXb7WmIrbqrhVnPgiGsJDWBv+lQOr+pMVDhB8dmVsXKM1V+jaGK/0Gotrhzs4GjjOh8qt739PEOiB3bifCwGidYfCFczFSxJkDkvnjE1xuUaaO/utKSUk5eY30lJ2izMBpRAshMX9UMaB6RRUqb48spwKgQMRantCklhARdNKp5QfoDZP12oEpzCt6KxyylFySSMMhigtkyiY3aORGBhqfhoVhe/W86in4UJ/DtM5fH4skW8gR1m6S5xv1HzTx4vklrPONklG7yqWTi/CbpcOK5fljOPMD4=","errorMsg":""}"#;
const WEEKLY_INFO_RESPONSE: &str = r#"{"code":200,"data":"s3JacFJkIjfxT8KZixht7mCnIQL8myvbNYClcviikWzYOsT2NdMeN+EuwjPKMVU8HzOHtRr2XqEJGApYtBUuNOEq50xL/0yJefcPMquiYYnZBrqlTJAHWgSmrMMH3yDr8vJqwH6JVmyAc4srg2R7oQ==","errorMsg":""}"#;
const WEEKLY_RESPONSE: &str = r#"{"code":200,"data":"+aoYiKT9eXYmCJNRHPcyLV7GYvVaX+rrT9Gwiu89BCqdhTnNHXOTqmkFp9HrL6GgGYR2ejjm1emdMHuoE0Y0EqSFkem7wuwsAAKtRF3kME6+bQHGgnByuh6ZbL9Xmru6uMXzGLzWOPwavC8mdhaSNjzMxCRUI2RExQq5FdcxipNkS7pG4xjS8CHMZenEJR7Z9ZeC8iMTzhWXCfrf3QPwdP79z+aia3KrF27jbUZtUBv84uF+yt6PMXwDzsmV8ymu7QERSJXwO6JQKZJFBSmbHU2noZF8RLT35rnWEiL6+DhjP9M/l9IieUkFVt9CYCfhcpRtq0DhtVHC2iTqTofls8nqf3pZJKNAHtqbzIRed1k=","errorMsg":""}"#;
const SCRAMBLE_RESPONSE: &str = "var scramble_id = 300;";

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
    ) -> impl std::future::Future<Output = hmanga_core::PluginResult<HttpResponse>> + Send {
        let responses = self.responses.clone();
        let requests = self.requests.clone();
        async move {
            requests.lock().unwrap().push(request);
            responses
                .lock()
                .unwrap()
                .pop_front()
                .ok_or_else(|| hmanga_core::PluginError::Other("missing fake response".to_string()))
        }
    }

    fn log(&self, _level: LogLevel, _msg: &str) {}
}

fn ok_response(body: &str) -> HttpResponse {
    HttpResponse {
        status: 200,
        headers: HashMap::new(),
        body: body.as_bytes().to_vec(),
    }
}

#[test]
fn jm_plugin_meta_exposes_supported_capabilities() {
    let plugin = JmPlugin::default();
    let meta = plugin.meta();

    assert_eq!(meta.id, "jm");
    assert!(meta.capabilities.search);
    assert!(meta.capabilities.login);
    assert!(meta.capabilities.favorites);
    assert!(meta.capabilities.weekly);
}

#[tokio::test]
async fn search_maps_encrypted_jm_payload_into_generic_comics() {
    let host = FakeHost::with_responses(vec![ok_response(SEARCH_RESPONSE)]);
    let plugin = JmPlugin::default().with_fixed_timestamp(1_712_688_000);

    let result = plugin
        .search(&host, "momo", 1, SearchSort::Latest)
        .await
        .unwrap();

    assert_eq!(result.current_page, 1);
    assert_eq!(result.total_pages, 1);
    assert_eq!(result.comics.len(), 1);
    assert_eq!(result.comics[0].id, "123");
    assert_eq!(result.comics[0].source, "jm");
    assert_eq!(result.comics[0].title, "标题A");
    assert_eq!(result.comics[0].author, "作者A");
    assert!(result.comics[0].cover_url.contains("123_3x4.jpg"));
    assert!(result.comics[0].tags.iter().any(|tag| tag == "同人"));

    let requests = host.requests();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].method, HttpMethod::Get);
    assert!(requests[0].url.contains("/search"));
    assert!(requests[0].headers.contains_key("token"));
}

#[tokio::test]
async fn get_comic_maps_series_into_generic_chapters() {
    let host = FakeHost::with_responses(vec![ok_response(COMIC_RESPONSE)]);
    let plugin = JmPlugin::default().with_fixed_timestamp(1_712_688_000);

    let comic = plugin.get_comic(&host, "123").await.unwrap();

    assert_eq!(comic.id, "123");
    assert_eq!(comic.title, "标题A");
    assert_eq!(comic.chapters.len(), 2);
    assert_eq!(comic.chapters[0].id, "456");
    assert_eq!(comic.chapters[0].title, "第1话 第一话");
    assert!(comic.tags.iter().any(|tag| tag == "校园"));
    assert_eq!(comic.extra.get("likes").map(String::as_str), Some("50"));
}

#[tokio::test]
async fn get_chapter_images_keeps_jm_specific_image_metadata_in_plugin_layer() {
    let host = FakeHost::with_responses(vec![
        ok_response(SCRAMBLE_RESPONSE),
        ok_response(CHAPTER_RESPONSE),
    ]);
    let plugin = JmPlugin::default().with_fixed_timestamp(1_712_688_000);

    let images = plugin.get_chapter_images(&host, "456").await.unwrap();

    assert_eq!(images.len(), 2);
    assert_eq!(images[0].index, 0);
    assert!(images[0].url.contains("/media/photos/456/00001.webp"));
    assert_eq!(
        images[0]
            .headers
            .get("x-hmanga-jm-scramble-id")
            .map(String::as_str),
        Some("300")
    );
    assert_eq!(
        images[0]
            .headers
            .get("x-hmanga-jm-file-name")
            .map(String::as_str),
        Some("00001")
    );
    assert_eq!(
        images[1]
            .headers
            .get("x-hmanga-jm-block-num")
            .map(String::as_str),
        Some("0")
    );
}

#[tokio::test]
async fn login_and_profile_decode_into_session_and_profile() {
    let host = FakeHost::with_responses(vec![
        ok_response(PROFILE_RESPONSE),
        ok_response(PROFILE_RESPONSE),
    ]);
    let plugin = JmPlugin::default().with_fixed_timestamp(1_712_688_000);

    let session = plugin.login(&host, "demo", "secret").await.unwrap();
    let profile = plugin.get_user_profile(&host).await.unwrap();

    assert_eq!(session.username, "demo");
    assert_eq!(session.token, "s");
    assert_eq!(profile.username, "demo");
    assert_eq!(profile.level_name, "普通会员");
    assert!(profile.photo.contains("avatar.jpg"));

    let requests = host.requests();
    assert_eq!(requests.len(), 2);
    assert_eq!(requests[0].method, HttpMethod::Post);
    assert!(String::from_utf8_lossy(requests[0].body.as_ref().unwrap()).contains("username=demo"));
}

#[tokio::test]
async fn favorites_and_weekly_map_to_generic_results() {
    let host = FakeHost::with_responses(vec![
        ok_response(FAVORITES_RESPONSE),
        ok_response(WEEKLY_INFO_RESPONSE),
        ok_response(WEEKLY_INFO_RESPONSE),
        ok_response(WEEKLY_RESPONSE),
    ]);
    let plugin = JmPlugin::default().with_fixed_timestamp(1_712_688_000);

    let favorites = plugin.get_favorites(&host, 0, 1).await.unwrap();
    let weekly_info = plugin.get_weekly_info(&host).await.unwrap();
    let weekly = plugin.get_weekly(&host, "101", "0").await.unwrap();

    assert_eq!(favorites.current_page, 1);
    assert_eq!(favorites.total_pages, 1);
    assert_eq!(favorites.folder_name.as_deref(), Some("默认"));
    assert_eq!(favorites.comics.len(), 1);
    assert_eq!(favorites.comics[0].title, "收藏漫画");

    assert_eq!(weekly_info.categories.len(), 1);
    assert_eq!(weekly_info.categories[0].title, "本周");
    assert_eq!(weekly_info.types[0].id, "0");

    assert_eq!(weekly.title, "本周 / 全部");
    assert_eq!(weekly.comics.len(), 1);
    assert_eq!(weekly.comics[0].id, "234");
    assert!(weekly.comics[0].tags.iter().any(|tag| tag == "韩漫"));
}
