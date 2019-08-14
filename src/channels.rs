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
    fn get(
        number: u64,
        pagination: Option<String>,
    ) -> Result<ChannelResponse, Box<std::error::Error>> {
        let mut params: Vec<(&str, String)> = vec![("first", number.to_string())];
        if let Some(page) = pagination {
            params.push(("after", page));
        }
        Request::request("streams", params)
    }
}

struct ChannelPages {
    page: Option<String>,
    number: u64,
}

impl Iterator for ChannelPages {
    type Item = ChannelResponse;

    fn next(&mut self) -> Option<ChannelResponse> {
        if self.number == 0 {
            return None;
        }
        let to_get = std::cmp::min(MAX_PER_PAGE, self.number);
        self.number = self.number.saturating_sub(to_get);
        match ChannelResponse::get(to_get, self.page.clone()) {
            Ok(r) => {
                self.page = Some(r.pagination.cursor.clone());
                Some(r)
            }
            Err(_) => None,
        }
    }
}

//found on twitch.tv by looking at network requests in dev tools
const CLIENT_ID: &str = "kimne78kx3ncx6brgo4mv6wki5h1ko";
const API_URL: &str = "https://api.twitch.tv/helix/";
const MAX_PER_PAGE: u64 = 100;
//TODO- lazy reusable request builder for best performance
//probably need to make this a struct for that
trait Request {
    fn request(endpoint: &str, params: Vec<(&str, String)>) -> Result<Self, Box<std::error::Error>>
    where
        Self: std::marker::Sized + serde::de::DeserializeOwned,
    {
        let url = reqwest::Url::parse_with_params(&(API_URL.to_owned() + endpoint), &params)?;
        let res = reqwest::Client::new()
            .get(&url.into_string())
            .header("Client-ID", CLIENT_ID)
            .send()?
            .json()?;
        Ok(res)
    }
}

impl Request for ChannelResponse {}

impl Request for UserResponse {}

pub fn top_connections(number: u64) -> Vec<String> {
    let mut logins: Vec<String> = Vec::with_capacity(number as usize);
    for page in (ChannelPages { page: None, number }) {
        let ids: Vec<String> = page.data.into_iter().map(|x| x.user_id).collect();
        // The ChannelPages iterator already returns up to the max of this endpoint anyway so it's
        // OK to keep this in the loop
        let resp = UserResponse::get_login_names(ids).unwrap();
        let mut l: Vec<String> = resp
            .data
            .into_iter()
            .map(|mut u| {
                u.login.insert_str(0, "#");
                u.login
            })
            .collect();

        logins.append(&mut l);
    }
    logins
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
    fn get_login_names(userids: Vec<String>) -> Result<UserResponse, Box<std::error::Error>> {
        let params: Vec<(&str, String)> = userids.into_iter().map(|s| ("id", s)).collect();
        Request::request("users", params)
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
