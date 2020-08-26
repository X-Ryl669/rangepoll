use std::fs;
use crate::rp_error::RPError;
use crate::voters;
use crate::poll;
use std::collections::HashMap;

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct Admin {
    pub voters: Vec<voters::Voter>,
    pub polls: Vec<poll::Poll>,
    pub inv_name: HashMap<String, String>,
    pub admin: String,
}

impl Admin {
    pub fn new(voter: &str) -> Admin
    {
        let mut adm = Admin { 
                voters: voters::get_voter_list().unwrap_or(Vec::new()), 
                polls: poll::get_poll_list().unwrap_or(Vec::new()),
                admin: voter.to_string(),
                inv_name: HashMap::new(),
            };
        for voter in &adm.voters {
            adm.inv_name.insert(voter.name.clone(), voter.filename.as_ref().unwrap().clone());
        }
        return adm;
    }
}

pub fn get_admin(voter: &str) -> Admin {
    return Admin::new(voter);
}
