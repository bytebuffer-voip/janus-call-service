use dashmap::DashMap;
use std::collections::HashSet;
use tokio::sync::mpsc::UnboundedSender;
use uuid::Uuid;

#[derive(Clone, Debug)]
pub struct ClientInfo {
    pub user_id: String,
    pub name: String,
    pub client_id: Uuid,
    pub device_id: String,
    pub ip: String,
    pub sender: UnboundedSender<String>,
}

#[derive(Default)]
pub struct ConnectionState {
    clients: DashMap<Uuid, ClientInfo>,
    user_map: DashMap<String, HashSet<Uuid>>,
}

impl ConnectionState {
    pub fn count_total_connections(&self) -> usize {
        self.clients.len()
    }

    pub fn count_online_users(&self) -> usize {
        self.user_map.len()
    }

    pub fn list_clients(&self, user_id: &str) -> Vec<ClientInfo> {
        if let Some(set) = self.user_map.get(user_id) {
            set.iter()
                .filter_map(|cid| self.clients.get(cid).map(|c| c.clone()))
                .collect()
        } else {
            vec![]
        }
    }

    pub fn check_user_online(&self, user_id: &str) -> bool {
        self.user_map.contains_key(user_id)
    }

    pub fn send_to_client_by_id(&self, client_id: &Uuid, data: String) {
        if let Some(client) = self.clients.get(client_id) {
            let _ = client.sender.send(data);
        }
    }

    pub fn send_to_user_except_client_id(
        &self,
        user_id: &str,
        except_client_id: &Uuid,
        data: String,
    ) {
        if let Some(set) = self.user_map.get(user_id) {
            for client_id in set.iter() {
                if client_id != except_client_id {
                    if let Some(client) = self.clients.get(client_id) {
                        let _ = client.sender.send(data.clone());
                    }
                }
            }
        }
    }

    pub fn send_to_user(&self, user_id: &str, data: String) {
        if let Some(set) = self.user_map.get(user_id) {
            for client_id in set.iter() {
                if let Some(client) = self.clients.get(client_id) {
                    let _ = client.sender.send(data.clone());
                }
            }
        }
    }

    pub fn add_client(&self, client_id: Uuid, info: ClientInfo) {
        let user_id = info.user_id.clone();
        self.clients.insert(client_id, info);
        self.user_map
            .entry(user_id.clone())
            .or_default()
            .insert(client_id);
    }

    pub fn remove_client(&self, client_id: &Uuid) {
        if let Some((_, client_info)) = self.clients.remove(client_id) {
            let user_id = &client_info.user_id;
            if let Some(mut entry) = self.user_map.get_mut(user_id) {
                entry.remove(&client_id);
                let _ = entry.is_empty();
                drop(entry);
            }
        }
    }
}

