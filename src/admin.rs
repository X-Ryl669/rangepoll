use crate::rp_error::RPError;
use crate::voters;
use crate::poll;
use crate::config;
use std::collections::HashMap;
extern crate lettre;


use lettre::sendmail::SendmailTransport;
use lettre::SmtpClient;
use lettre_email::Email;
use lettre::smtp::authentication::{ Credentials, Mechanism };
use lettre::smtp::ConnectionReuseParameters;
use url::{ Url };
use rocket_contrib::templates::tera;

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct VoterMap {
    pub fullname: String,
    pub email:    String,
    pub filestem: String,
}

impl VoterMap {
    pub fn new(voter: &voters::Voter) -> VoterMap {
        VoterMap { 
            fullname: voter.fullname.as_ref().unwrap_or(&voter.username).clone(),
            email: voter.email.as_ref().unwrap_or(&"".to_string()).clone(),
            filestem: voter.filename.as_ref().unwrap_or(&"".to_string()).clone(),
        }
    }
}


#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct Admin {
    pub voters: Vec<voters::Voter>,
    pub polls: Vec<poll::Poll>,
    pub inv_name: HashMap<String, VoterMap>,
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
            adm.inv_name.insert(voter.username.clone(), VoterMap::new(&voter));
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

pub fn update_poll(cfg: Option<&config::Config>, actor: &str, action: &str, poll_filename: &str, poll: Option<&poll::Poll>) -> Result<bool, RPError> {
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
        "edit" => {
            return Err(RPError::from(std::io::Error::new(std::io::ErrorKind::NotFound, format!("{} not connected", action))));
        },
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
            if cfg.is_none() || cfg.unwrap().smtp_server.is_none() {
                return Err(RPError::from(std::io::Error::new(std::io::ErrorKind::InvalidInput, format!("No configuration for mail sending"))));
            }
            // Collect all emails for each voter and send them an email if valid
            let tokens = poll::gen_voters_token(poll_filename)?;
            let poll_desc = poll::get_poll_desc(poll_filename, false)?;
            return send_emails(cfg.unwrap(), admin, tokens, &poll_desc);
        },
        _ => Err(RPError::from(std::io::Error::new(std::io::ErrorKind::NotFound, format!("{} not found", action))))
    } 
}

fn send_email_impl<'a>( admin: &Admin, 
                        sender: &str, 
                        tokens: &Vec<poll::Token>, 
                        poll_desc: &poll::ParsedPoll, 
                        subject: &str, 
                        base_url: &str,
                        transport: &mut impl lettre::Transport::<'a> ) -> Result<bool, RPError> {
    let tera = match tera::Tera::new("templates/*.smtp.tera") {
        Ok(v) => v,
        Err(e) => { return Err(RPError::from(std::io::Error::new(std::io::ErrorKind::Other, format!("Tera engine error: {}", e)))); }
    };

    let mut context: HashMap<&str, String> = HashMap::new();
    context.insert("inviter", admin.admin.clone());
    context.insert("pollname", poll_desc.name.clone());
    context.insert("deadline", poll_desc.deadline_date.clone());
    context.insert("polldesc", poll_desc.desc.clone());
    context.insert("logourl", format!("{}public/css/logo.png", base_url));

    for token in tokens.iter() {
        if !admin.inv_name.contains_key(&token.voter) {
            println!("Failed to find a valid email for {}", token.voter);
            continue;
        }
        
        let voter_map = admin.inv_name.get(&token.voter).unwrap();
        // Let's build the tera context
        context.insert("link", format!("{}token/{}", base_url, token.token));
        context.insert("fullname", voter_map.fullname.clone());

        // Then generate both HTML and text version for the email
        let mut html = tera.render("invite_html.smtp.tera", &context);
        let txt = tera.render("invite_text.smtp.tera", &context);
        if txt.is_err() {
            eprintln!("Failed to render SMTP's textual invite message from invite_text.smtp.tera");
            continue;
        }
        if html.is_err() {
            html = Ok(txt.as_ref().unwrap().clone());
        }

        // Then format the email to send
        let email = Email::builder()
                        // Addresses can be specified by the tuple (email, alias)
                        .to((&voter_map.email, &voter_map.fullname))
                        // ... or by an address only
                        .from(sender)
                        .subject(subject)
                        .alternative(html.unwrap(), txt.unwrap())
                        .build();

        if email.is_err() {
            return Err(RPError::from(std::io::Error::new(std::io::ErrorKind::InvalidData, format!("SMTP email error: {:?}", email.err()))));
        }
        
        transport.send(email.unwrap().into());
/*        match result {
            Ok(_) => { println!("Send email to {}", voterMap.email); },
            Err(e) => { eprintln!("Failed to send email {}", e); },
        }
        */
    }
    return Ok(true);
}

pub fn send_emails(cfg: &config::Config, admin: Admin, tokens: Vec<poll::Token>, poll_desc: &poll::ParsedPoll) -> Result<bool, RPError> {
    // Need to extract the sendmail configuration
  //  let mut recipients = Vec::new();
  //  let mut recipients_name = Vec::new();

    let host_url = match Url::parse(&format!("http://{}", cfg.smtp_sender.as_ref().unwrap_or(&"bad".to_string()))){
            Ok(u) => u,
            Err(_) => Url::parse(&cfg.base_url).unwrap(),
        };

    let host = host_url.host_str().unwrap_or("localhost").to_string();
    let no_reply_default = format!("no_reply@{}", host).clone();
    let sender = cfg.smtp_sender.as_ref().unwrap_or(&no_reply_default);
    let subject = cfg.smtp_invite_subject.as_ref().unwrap_or(&"Invitation to vote".to_string()).clone();
    let base_url = format!("{}/", cfg.base_url);

    // Either build a SMTP transport or use system's sendmail 
    if cfg.smtp_server.as_ref().unwrap() == "sendmail" {
        let mut transport = SendmailTransport::new();
        return send_email_impl(&admin, &sender, &tokens, poll_desc, &subject, &base_url, &mut transport);
    } else
    {
        let mut mailer = match SmtpClient::new_simple(cfg.smtp_server.as_ref().unwrap()) { //&format!("{}:{}", cfg.smtp_server.as_ref().unwrap(), match cfg.smtp_port { Some(v) => v, None => 25u16 })) {
            Ok(v) => v,
            Err(e) => { eprintln!("{}", e); return Err(RPError::from(std::io::Error::new(std::io::ErrorKind::InvalidInput, format!("SMTP client error: {}", e)))); },
        };

        // Enable SMTPUTF8 if the server supports it
        mailer = mailer.smtp_utf8(true)
                        // Configure expected authentication mechanism
                        .authentication_mechanism(Mechanism::Plain)
                        // Enable connection reuse
                        .connection_reuse(ConnectionReuseParameters::ReuseUnlimited);
        if cfg.smtp_username.is_some() {
            mailer = mailer.credentials(Credentials::new(cfg.smtp_username.as_ref().unwrap().clone(), cfg.smtp_password.as_ref().unwrap_or(&"".to_string()).clone()));
        }
        let mut transport = mailer.transport();
        let res = send_email_impl(&admin, &sender, &tokens, poll_desc, &subject, &base_url, &mut transport);
        transport.close();
        return res;
    };
}