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

pub fn update_voter(actor: &str, action: &str, voter_name: &str, voter: Option<&voters::Voter>) -> Result<bool, RPError> {
    // Check if the current user is admin too
    let admin = get_admin(actor);
    let cur_user_is_admin = match admin.voters.iter().filter(|&x| x.name == actor).next() 
        {
            Some(v) => v.admin,
            None => false
        };
    if !cur_user_is_admin {
        return Err(RPError::from(std::io::Error::new(std::io::ErrorKind::PermissionDenied, format!("{} is not an admin", actor))));
    }

    match action.to_ascii_lowercase().as_str()
    {
        "delete" => Ok(voters::delete_voter(&voter_name)),
        "update" => {
            if voter.is_none() {
                return Err(RPError::from(std::io::Error::new(std::io::ErrorKind::InvalidInput, format!("{} is empty", actor))));
            }
            Ok(voters::update_voter(&voter_name, voter.unwrap()))
        },
        _ => Err(RPError::from(std::io::Error::new(std::io::ErrorKind::NotFound, format!("{} not found", action))))
    } 

}