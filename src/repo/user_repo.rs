use crate::config::config::Config;
use crate::config::constants::COLLECTION_USERS;
use crate::config::mongodb_cfg::get_mongo_client;
use futures_util::TryStreamExt;
use mongodb::bson;
use mongodb::bson::doc;
use crate::model::user::User;

pub async fn get_user(user_id: &str) -> anyhow::Result<Option<User>> {
    if user_id.is_empty() {
        return Ok(None);
    }
    let cfg = Config::global_config();
    let client = get_mongo_client().await?;
    let collection = client
        .database(cfg.mongodb.db_name.as_str())
        .collection::<User>(COLLECTION_USERS);
    let filter = doc! {"id": user_id };
    let wp = collection.find_one(filter).await?;
    Ok(wp)
}

pub async fn get_users_except(user_id: &str, keyword: Option<&str>) -> anyhow::Result<Vec<User>> {
    let cfg = Config::global_config();
    let client = get_mongo_client().await?;
    let collection = client
        .database(cfg.mongodb.db_name.as_str())
        .collection::<User>(COLLECTION_USERS);
    let mut filter = doc! { "id": { "$ne": user_id } };
    if let Some(key) = keyword {
        if !key.is_empty() {
            let regex = doc! { "$regex": key, "$options": "i" };
            filter.insert(
                "$or",
                bson::to_bson(&vec![
                    doc! { "username": &regex },
                    doc! { "first_name": &regex },
                    doc! { "last_name": &regex },
                ])?,
            );
        }
    }
    let cursor = collection.find(filter).await?;
    let users: Vec<User> = cursor.try_collect().await?;
    Ok(users)
}

pub async fn get_all_users() -> anyhow::Result<Vec<User>> {
    let cfg = Config::global_config();
    let client = get_mongo_client().await?;
    let collection = client
        .database(cfg.mongodb.db_name.as_str())
        .collection::<User>(COLLECTION_USERS);
    let cursor = collection.find(doc! {}).await?;
    let users: Vec<User> = cursor.try_collect().await?;
    Ok(users)
}

pub async fn search_users(keyword: &str) -> anyhow::Result<Vec<User>> {
    let cfg = Config::global_config();
    let client = get_mongo_client().await?;
    let collection = client
        .database(cfg.mongodb.db_name.as_str())
        .collection::<User>(COLLECTION_USERS);
    let regex = doc! { "$regex": keyword, "$options": "i" };
    let filter = doc! {
        "$or": [
            { "username": &regex },
            { "first_name": &regex },
            { "last_name": &regex },
        ]
    };
    let cursor = collection.find(filter).await?;
    let users: Vec<User> = cursor.try_collect().await?;
    Ok(users)
}
