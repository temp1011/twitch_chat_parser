use serde::{Deserialize, Serialize};
//TODO - I can't seem to find a way to do this with the new API.
const API_URL_v5: &str = "https://api.twitch.tv/v5";

const CLIENT_ID: &str = "kimne78kx3ncx6brgo4mv6wki5h1ko";
//const MAX_PER_PAGE: u64 = 100;
//TODO- lazy reusable request builder for best performance
//probably need to make this a struct for that
trait Request {
    fn request(endpoint: &str, params: Vec<(&str, String)>) -> Result<Self, Box<std::error::Error>>
    where
        Self: std::marker::Sized + serde::de::DeserializeOwned,
    {
        let url = reqwest::Url::parse_with_params(&(API_URL_v5.to_owned() + endpoint), &params)?;
        let res = reqwest::Client::new()
            .get(&url.into_string())
            .header("Client-ID", CLIENT_ID)
            .send()?
            .json()?;
        Ok(res)
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct CommentsJson {
    _next: String,
    comments: Vec<CommentsBody>,
}

#[derive(Serialize, Deserialize, Debug)]
struct CommentsBody {
    _id: String,
    channel_id: String,
    commenter: CommentUser,
    message: CommentMessage,
}

#[derive(Serialize, Deserialize, Debug)]
struct CommentUser {
    _id: String,
    name: String,
    //TODO type
    updated_at: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct CommentMessage {
    body: String,
    user_badges: Option<Vec<CommentMessageBadges>>,
    emoticons: Option<Vec<CommentEmotes>>,
}

//ugh so maany structs
#[derive(Serialize, Deserialize, Debug)]
struct CommentMessageBadges {
    _id: String,
    version: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct CommentEmotes {
    _id: String,
    begin: u16,
    end: u16,
}

impl Request for CommentsJson {}

#[test]
fn test_comments_json() {
    let params: Vec<(&str, String)> = vec![("cursor", "".to_string())];
    let resp = CommentsJson::request("/videos/459581393/comments", params).unwrap();
}
