#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use] extern crate rocket;
#[macro_use] extern crate serde_derive;
extern crate clap;

use std::path::{ Path, PathBuf };
use std::collections::HashMap;
use std::env;
use rocket::response::NamedFile;
use rocket_contrib::serve::StaticFiles;
use rocket_contrib::templates::Template;
use rocket::http::{ Cookies, Cookie, Status };
use rocket::response::{ Flash, Redirect };
use rocket::request::{ Form, LenientForm };
use rocket::config::{ Config, Environment };
use rocket::response::status::Custom;

// Let authentication be checked with Request Guard
use rocket::request::{ self, Request, FromRequest };
use clap::{ App, Arg };
use std::net::IpAddr;


mod poll;
mod voters;

#[derive(FromForm)]
struct User {
    name: String,
    password: String,
}

#[derive(Debug)]
struct Voter {
    name: String,
}

impl<'a, 'r> FromRequest<'a, 'r> for Voter {
    type Error = ();

    fn from_request(request: &'a Request<'r>) -> request::Outcome<Voter, ()> {
        let mut cookies = request.cookies();
        match cookies.get_private("auth") {
            Some(cookie) => request::Outcome::Success(Voter{ name: cookie.value().to_string()}),
            None => request::Outcome::Forward(())
        }
    }
}


// Extract the "unknown" field name as vote only if they are integers
impl<'f> request::FromForm<'f> for poll::poll::VotesForVoter {
    type Error = ();

    fn from_form(form_items: &mut request::FormItems<'f>, _: bool) -> Result<Self, ()> {
        let mut votes = poll::poll::VotesForVoter {
            name: String::new(),
            votes: HashMap::new(),
        };

        for (key, value) in form_items.map(|i| i.key_value_decoded()) {
            if &*key == "name" {
                votes.name = value;
            }
            else {
                match value.parse() {
                    Ok(n) => votes.votes.insert(key, n),
                    // Ignore invalid votes anyway
                    Err(_) => None,
                };
            }
        }

        Ok(votes)
    }
}

#[derive(Serialize)]
struct LoginContext {
    site_name: &'static str,
    dest: &'static str
}



#[get("/token/<token>")]
fn log_with_token(mut cookies: Cookies, token: String) -> Result< Redirect, Custom<Template> > {
    // Using JWT token here for authentication 
    let voter = match poll::poll::validate_token(&token) {
        Ok(v) => v,
        Err(e) => {
            println!("Error ({}) with token: {}", e, token);
            let mut ctx = HashMap::new();
            ctx.insert("msg", "Invalid credentials");
            return Err(Custom(Status::Unauthorized, Template::render("error/401", ctx)));
        }
    };

    cookies.add_private(Cookie::new("auth", voter.1.clone()));
    cookies.add(Cookie::build("user", voter.1.clone()).path("/").finish());
    return Ok(Redirect::to(format!("/?vote={}", voter.0.clone()))); 
}

#[get("/login")]
fn login() -> Template { 
    let context = LoginContext { site_name: "range poll", dest: "/login" };
    Template::render("login", &context)
}

#[post("/login", data = "<user>")]
fn post_login(mut cookies: Cookies, user: LenientForm<User>) -> Result< Redirect, Custom<Template> > {
    let voters = match voters::voters::get_voter_list() {
        Ok(v) => v,
        Err(_) => Vec::new(),
    };

    if voters.len() == 0 {
        let mut ctx = HashMap::new();
        ctx.insert("msg", "No voter declared yet");
        return Err(Custom(Status::MisdirectedRequest, Template::render("error/421", ctx)));
    }
    for voter in voters {
        if user.name.to_lowercase() == voter.name.to_lowercase() && user.password == voter.password {
            cookies.add_private(Cookie::new("auth", voter.name.clone()));
            cookies.add(Cookie::new("user", voter.name.clone()));
            return Ok(Redirect::to("/poll_list")); 
        } 
    }
    let mut ctx = HashMap::new();
    ctx.insert("msg", "Invalid credentials");
    return Err(Custom(Status::Unauthorized, Template::render("error/401", ctx)));
}

/// Remove the `auth` cookie.
#[get("/logout")]
fn logout(mut cookies: Cookies) -> Flash<Redirect> {
    cookies.remove_private(Cookie::named("auth"));
    cookies.remove(Cookie::named("user"));
    Flash::success(Redirect::to("/"), "Successfully logged out.")
}
/*
#[get("/admin")]
fn admin_panel(admin: AdminUser) -> &'static str {
    "Hello, administrator. This is the admin panel!"
}

#[get("/admin", rank = 2)]
fn admin_panel_user(user: User) -> &'static str {
    "Sorry, you must be an administrator to access this page."
}

#[get("/admin", rank = 3)]
fn admin_panel_redirect() -> Redirect {
    Redirect::to(uri!(login))
}
*/


#[get("/")]
fn index() -> Option<NamedFile> {
    NamedFile::open(Path::new("static/html/index.html")).ok()
}

#[catch(404)]
fn not_found(req: &Request) -> Template {
    let mut map = HashMap::new();
    map.insert("path", req.uri().path());
    Template::render("error/404", &map)
}

// Asynchronous javascript methods here
#[get("/poll_list", rank=1)]
fn poll_list(voter: Voter) -> Result<Template, Flash<Redirect>> {
    let mut map = HashMap::new();
    // Need to extract all available polls
    let polls = match poll::poll::get_poll_desc_list(&voter.name) {
        Ok(v) => v,
        Err(_) => { map.insert("polls", vec![]); return Ok(Template::render("poll_list", &map)); },
    };
    map.insert("polls", polls);
    Ok(Template::render("poll_list", &map))
}
#[get("/poll_list", rank=2)]
fn poll_list_not_logged() -> Flash<Redirect> {
    Flash::error(Redirect::to("/login"), "Invalid credentials")
}

#[get("/vote_for/<poll>", rank=1)]
fn vote_for(poll: String, voter: Voter) -> Result<Template, Flash<Redirect>> {
    // Need to extract all available polls
    let mut ppoll = match poll::poll::get_poll_desc(&poll) {
        Ok(v) => v,
        Err(_) => { 
            let mut ctx = HashMap::new();
            ctx.insert("msg", "No poll found on server");
            return Ok(Template::render("error/412", &ctx)); 
        },
    };
    if !ppoll.allowed_participant.contains(&voter.name) {
        return Err(Flash::error(Redirect::to("/not_allowed/poll_list/".to_owned() + &poll), "Not allowed for you to vote"));
    } else {
        ppoll.user = voter.name.clone();
    }
    
    Ok(Template::render("vote_for", &ppoll))
}
#[post("/vote_for/<poll>", rank=1, data="<form>")]
fn post_vote_for(poll: String, voter: Voter, form: Form<poll::poll::VotesForVoter>) -> Result<Template, Flash<Redirect>> {
    // Don't trust the form submitter and only use the authentication token we have generated here for the voter's name.
    let vote = poll::poll::VotesForVoter { name: voter.name.clone(), votes: form.votes.clone() };
    let mut ppoll = match poll::poll::vote_for_poll(&poll, &vote) {
        Ok(v) => v,
        Err(e) => { return Err(Flash::error(Redirect::to(format!("/not_allowed/{}/{}", "/poll_list", poll)), format!("{:?}", e))); },
    };
/*    if !ppoll.allowed_participant.contains(&voter.name) {
        return Err(Flash::error(Redirect::to("/not_allowed/poll_list/".to_owned() + &poll), "Not allowed for you to vote"));
    } else {
    }
*/    
    ppoll.user = voter.name.clone();
    Ok(Template::render("vote_result", &ppoll))
}

#[get("/vote_for/<_poll>", rank=2)]
fn vote_for_not_logged(_poll: String) -> Flash<Redirect> {
    Flash::error(Redirect::to("/login"), "Invalid credentials")
}
#[get("/not_allowed/<dest>/<from>")]
fn not_allowed(dest: String, from: String) -> Template {
    let mut map = HashMap::new();
    map.insert("dest", dest);
    map.insert("from", from);
    return Template::render("not_allowed", &map);
}

#[get("/vote_results/<dest>", rank=1)]
fn vote_results(dest: String, voter: Voter) -> Result<Template, Flash<Redirect>> {
    // Need to extract vote results for the given name
    let mut pollr = match poll::poll::get_poll_result(dest.as_str(), voter.name.clone()) {
        Ok(v) => v,
        Err(e) => { return Err(Flash::error(Redirect::to(format!("/not_allowed/{}/{}", "/poll_list", dest)), format!("{:?}", e))); },
    };
    
    pollr.user = voter.name.clone();
    Ok(Template::render("vote_result", &pollr))
}
#[get("/vote_results/<_dest>", rank=2)]
fn vote_results_not_logged(_dest: String) -> Flash<Redirect> {
    Flash::error(Redirect::to("/login"), "Invalid credentials")
}

#[get("/menu", rank=1)]
fn menu(voter: Voter) -> Template {
    let mut ctx = HashMap::new();
    ctx.insert("name", voter.name.clone());
    Template::render("menu", &ctx)
}
#[get("/menu", rank=2)]
fn menu_not_logged() -> Flash<Redirect> {
    Flash::error(Redirect::to("/login"), "Invalid credentials")
}


/*
#[get("/login")]
fn login(cookies: Cookies) -> &'static str {
    let auth_token = cookies.get("auth");
    if (!is_cookie_valid(auth_token.value())) {
        ""
    }
}
*/

// Temporary stuff below
#[get("/static/<file..>")]
fn static_files(file: PathBuf) -> Option<NamedFile> {
    NamedFile::open(Path::new("static/").join(file)).ok()
}


fn main() {
    // Start main backend
    let mut port: u16 = 8000;
    let mut host = "localhost".to_string();

    let cmd_args = App::new("rangepoll")
                        .version("0.1.0")
                        .author("Cyril R. <boite.pour.spam@gmail.com>")
                        .about("A voting server to run poll with weighted choices and collecting results")
                        .arg(Arg::with_name("port")
                               .short("p")
                               .long("port")
                               .value_name("port number")
                               .help("Sets the webserver port to listen to")
                               .default_value("8000")
                               .takes_value(true))
                        .arg(Arg::with_name("host").short("a").long("host").value_name("hostname").help("Specify the webserver hostname").default_value("localhost").takes_value(true))
                        .arg(Arg::with_name("poll").short("g").long("gen-template").value_name("template.yml").help("Generate a template poll YAML file and save to template.yaml (recommanded: polls/example.yml)").takes_value(true))
                        .arg(Arg::with_name("voter").short("v").long("gen-voter").value_name("voter.yml").help("Generate a template voter YAML file and save to voter.yaml (recommanded: voters/voter.yml)").takes_value(true))
                        .arg(Arg::with_name("token").short("t").long("gen-token").value_name("poll name").help("Generate tokens for the given poll's voters so it can be distributed by email for example").takes_value(true))
                        .get_matches();
    
    if let Some(o) = cmd_args.value_of("port") {
        port = o.parse().unwrap_or(8000);
    }
    if let Some(o) = cmd_args.value_of("host") {
        host = o.to_string();
    }
    if let Some(o) = cmd_args.value_of("poll") {
        poll::poll::gen_template(o);
        println!("Generated template poll file to {:?}", o);
        return;
    }
    if let Some(o) = cmd_args.value_of("voter") {
        voters::voters::gen_template(o);
        println!("Generated template voter file to {:?}", o);
        return;
    }
    if let Some(o) = cmd_args.value_of("token") {
        let tokens = match poll::poll::gen_voters_token(o) {
            Ok(v) => v,
            Err(e) => { eprintln!("Error: {}", e); return; }
        };

        let max_voter_name = tokens.iter().map(|x| x.voter.len()).max().unwrap();

        println!("{:width$}    Token", "Voter", width = max_voter_name);
        for token in tokens {
            println!("{:width$}    http://{}:{}/token/{}", token.voter, host, port, token.token, width = max_voter_name);
        }
        return;
    }


    let host_interface = host.parse::<IpAddr>().unwrap_or("0.0.0.0".parse::<IpAddr>().unwrap());

    let config = Config::build(Environment::Staging)
                        .address(host_interface.to_string())
                        .port(port)
                        .finalize().expect("Error building webserver config");

    let r = rocket::custom(config);

//  Enable this if you prefer to use a rocket.toml file for configuration
//    let r = rocket::ignite();

    r.attach(Template::fairing())
     .mount("/", routes![index])
     .mount("/static", routes![static_files])
     // Ajax below
     // Login or logout
     .mount("/", routes![login, post_login, logout, not_allowed, log_with_token])
     // Asynchronous application
     .mount("/", routes![poll_list, vote_for, post_vote_for, vote_results, menu])
     // Not logged in async routes
     .mount("/", routes![poll_list_not_logged, vote_for_not_logged, vote_results_not_logged, menu_not_logged])

     // Static below
     .mount("/public", StaticFiles::from(concat!(env!("CARGO_MANIFEST_DIR"), "/static")))
     .register(catchers![not_found])
     .launch();
}

