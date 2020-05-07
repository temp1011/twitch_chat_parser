use futures::stream::{self, StreamExt};
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
struct Pagination {
    cursor: String,
}

//other fields are included for now for interest. Maybe remove in the future because
//they don't serve much use.
#[derive(Serialize, Deserialize, Debug)]
struct ChannelJson {
    community_ids: Option<Vec<String>>,
    game_id: String,
    id: String,
    language: String,
    started_at: String,
    tag_ids: Vec<String>,
    thumbnail_url: String,
    title: String,
    r#type: String,
    user_id: String,
    user_name: String,
    viewer_count: u32,
}

#[derive(Serialize, Deserialize, Debug)]
struct ChannelResponse {
    data: Vec<ChannelJson>,
    pagination: Pagination,
}

impl ChannelResponse {
    async fn get(
        number: u64,
        pagination: Option<String>,
    ) -> Result<ChannelResponse, Box<dyn std::error::Error>> {
        let mut params: Vec<(&str, String)> = vec![("first", number.to_string())];
        if let Some(page) = pagination {
            params.push(("after", page));
        }
        request("streams", params).await
    }
}

struct ChannelPages {
    page: Option<String>,
    number: u64,
}

async fn pages(channel_pages: ChannelPages) -> Option<(ChannelResponse, ChannelPages)> {
    if channel_pages.number == 0 {
        return None;
    }

    let to_get = std::cmp::min(MAX_PER_PAGE, channel_pages.number);

    //TODO use overflowing sub here with MAX_PER_PAGE?
    let new_to_get = channel_pages.number.saturating_sub(to_get);
    match ChannelResponse::get(to_get, channel_pages.page).await {
        Ok(r) => {
            let curs = r.pagination.cursor.clone();
            Some((
                r,
                ChannelPages {
                    page: Some(curs),
                    number: new_to_get,
                },
            ))
        }
        Err(_) => None,
    }
}

//found on twitch.tv by looking at network requests in dev tools
const CLIENT_ID: &str = "jzkbprff40iqj646a697cyrvl0zt2m6";
const API_URL: &str = "https://api.twitch.tv/helix/";
const MAX_PER_PAGE: u64 = 100;

lazy_static! {
    static ref CLIENT: reqwest::Client = {
        let mut header_map = reqwest::header::HeaderMap::new();
        header_map.insert("Client-ID", CLIENT_ID.parse().unwrap());

        reqwest::Client::builder()
            .default_headers(header_map)
            .build()
            .unwrap()
    };
}

//TODO- lazy reusable request builder for best performance
//probably need to make this a struct for that
async fn request<T>(
    endpoint: &str,
    params: Vec<(&str, String)>,
) -> Result<T, Box<dyn std::error::Error>>
where
    T: std::marker::Sized + serde::de::DeserializeOwned,
{
    let url = reqwest::Url::parse_with_params(&(API_URL.to_owned() + endpoint), &params)?;
    let res = CLIENT.get(&url.into_string()).send().await?.json().await?;
    Ok(res)
}

pub async fn top_connections(number: u64) -> Vec<String> {
    stream::unfold(ChannelPages { page: None, number }, pages)
        .then(|page| async move {
            let ids: Vec<String> = page.data.into_iter().map(|x| x.user_id).collect();
            // The ChannelPages iterator already returns up to the max of this endpoint anyway so it's
            // OK to keep this in the loop
            // TODO but it shouldn't unwrap
            let resp = UserResponse::get_login_names(ids).await.unwrap();
            stream::iter(resp.data.into_iter().map(|u| u.login))
        })
        .flatten()
        .collect::<Vec<String>>()
        .await
}

#[derive(Serialize, Deserialize, Debug)]
struct UserJson {
    broadcaster_type: String,
    description: String,
    display_name: String,
    id: String,
    login: String,
    offline_image_url: String,
    profile_image_url: String,
    r#type: String,
    view_count: u32,
}

#[derive(Serialize, Deserialize, Debug)]
struct UserResponse {
    data: Vec<UserJson>,
}

impl UserResponse {
    //TODO - Sometimes this seems to return fewer channels than requested. Maybe return an error
    //for this too
    async fn get_login_names(
        userids: Vec<String>,
    ) -> Result<UserResponse, Box<dyn std::error::Error>> {
        let params: Vec<(&str, String)> = userids.into_iter().map(|s| ("id", s)).collect();
        request("users", params).await
    }
}

#[test]
fn test_get_login_names() {
    let resp = UserResponse::get_login_names(vec!["23161357".to_string()]).unwrap();
    assert!(resp.data.len() == 1);
    assert_eq!(resp.data[0].display_name, "LIRIK");
}

#[test]
fn test_top_connections() {
    let resp = top_connections(10);
    assert_eq!(resp.len(), 10);
}

#[test]
fn test_channel_response() {
    let resp = ChannelResponse::get(4, None).unwrap();
    assert_eq!(4, resp.data.len());
}
