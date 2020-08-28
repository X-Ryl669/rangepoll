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
            adm.inv_name.insert(voter.username.clone(), voter.fullname.as_ref().unwrap_or(&voter.username).clone());
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
    let cur_user_is_admin = match admin.voters.iter().filter(|&x| x.username == actor).next() 
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

pub fn update_poll(actor: &str, action: &str, poll_filename: &str, poll: Option<&poll::Poll>) -> Result<bool, RPError> {
    // Check if the current user is admin too
    let admin = get_admin(actor);
    let cur_user_is_admin = match admin.voters.iter().filter(|&x| x.username == actor).next() 
        {
            Some(v) => v.admin,
            None => false
        };
    if !cur_user_is_admin {
        return Err(RPError::from(std::io::Error::new(std::io::ErrorKind::PermissionDenied, format!("{} is not an admin", actor))));
    }

    match action.to_ascii_lowercase().as_str()
    {
        "delete" => Ok(poll::delete_poll(&poll_filename)),
        "update" => {
            if poll.is_none() {
                return Err(RPError::from(std::io::Error::new(std::io::ErrorKind::InvalidInput, format!("{} is empty", actor))));
            }
            Ok(poll::update_poll(&poll_filename, poll.unwrap()))
        },
        "del_voter" => {
            // Extract the poll to update first
            let info: Vec<&str> = poll_filename.split(":").collect();
            if info.len() != 2 || !admin.inv_name.contains_key(info[1]) {
                return Err(RPError::from(std::io::Error::new(std::io::ErrorKind::InvalidInput, format!("{} is empty", actor))));
            }
            Ok(poll::del_voter_in_poll(info[0], info[1]))
        },
        "add_voter" => {
            // Extract the poll to update first
            let info: Vec<&str> = poll_filename.split(":").collect();
            if info.len() != 2 || !admin.inv_name.contains_key(info[1]) {
                return Err(RPError::from(std::io::Error::new(std::io::ErrorKind::InvalidInput, format!("{} is empty", actor))));
            }
            Ok(poll::add_voter_in_poll(info[0], info[1]))
        },
        "sendemail" => {
            Err(RPError::from(std::io::Error::new(std::io::ErrorKind::NotConnected, format!("{} not connected", action))))
        },
        _ => Err(RPError::from(std::io::Error::new(std::io::ErrorKind::NotFound, format!("{} not found", action))))
    } 

}