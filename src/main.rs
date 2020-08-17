#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use] extern crate rocket;
#[macro_use] extern crate serde_derive;


use std::path::{ Path, PathBuf };
use std::collections::HashMap;
use std::env;
use rocket::response::NamedFile;
use rocket_contrib::serve::StaticFiles;
use rocket_contrib::templates::Template;
use rocket::http::{ Cookies, Cookie };
use rocket::response::{ Flash, Redirect };
use rocket::request::{ Form, LenientForm };
use rocket::config::{ Config, Environment };

// Let authentication be checked with Request Guard
use rocket::request::{self, Request, FromRequest};

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

#[get("/login")]
fn login() -> Template { 
    let context = LoginContext { site_name: "range poll", dest: "/login" };
    Template::render("login", &context)
}

#[post("/login", data = "<user>")]
fn post_login(mut cookies: Cookies, user: LenientForm<User>) -> Flash<Redirect> {
    let voters = match voters::voters::get_voter_list() {
        Ok(v) => v,
        Err(_) => return Flash::error(Redirect::to("/"), "Invalid credentials"),
    };
    for voter in voters {
        if user.name == voter.name && user.password == voter.password {
            cookies.add_private(Cookie::new("auth", user.name.clone()));
            cookies.add(Cookie::new("user", user.name.clone()));
            return Flash::success(Redirect::to("/"), "Successfully logged in.");
        } 
    }
    Flash::error(Redirect::to("/"), "Invalid credentials")
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
fn poll_list(_voter: Voter) -> Result<Template, Flash<Redirect>> {
/*    let auth_token = cookies.get_private("auth");
    if auth_token.is_none() {
        Err(Flash::error(Redirect::to("/login"), "Invalid credentials"))
    } else {
        */
        let mut map = HashMap::new();
        // Need to extract all available polls
        let polls = match poll::poll::get_poll_desc_list() {
            Ok(v) => v,
            Err(_) => { map.insert("polls", vec![]); return Ok(Template::render("poll_list", &map)); },
        };
        map.insert("polls", polls);
        Ok(Template::render("poll_list", &map))
//    }
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
        Err(_) => { return Err(Flash::error(Redirect::to("/error/404"), "Poll not found")); },
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
        Err(e) => { return Err(Flash::error(Redirect::to("/error/404"), format!("{:?}", e))); },
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

fn help(process_name: &String) {
    println!(r#"Usage is: {:?} [command]
with [command] any of:
    -p 80            Port to listen to
    -g template.yaml Generate a template poll YAML file and save to template.yaml (recommanded: polls/example.yml)
    -v voter.yaml    Generate a template voter YAML file and save to template.yaml (recommanded: voters/users.yml)
"#, process_name)
}

fn main() {
    // Start main backend
    // Parse arguments
    let mut port: u16 = 8000;
    let args: Vec<String> = env::args().collect();
    match args.len() {
        // no arguments passed
        1 => {
            println!("Welcome to RangePoll server.");
        },
        // one argument passed
        2 => {
            help(&args[0]);
            return;
        },
        // one command and one argument passed
        3 => {
            let cmd = &args[1];
            let num = &args[2];
            // parse the command
            match &cmd[..] {
                "-p" => {
                    match num.parse() { 
                        Ok(n) => { port = n; },
                        Err(_) => { eprintln!("error: second argument not an integer"); help(&args[0]); return; }
                    };
                },
                "-g" => {
                    poll::poll::gen_template(num);
                    println!("Generated template poll file to {:?}", num);
                    return;
                },
                "-v" => {
                    voters::voters::gen_template(num);
                    println!("Generated template voter file to {:?}", num);
                    return;
                }
                _ => { help(&args[0]); return; }
            }
        },
        _ => { help(&args[0]); return; }
    }


    let config = Config::build(Environment::Staging)
                        .address("localhost")
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
     .mount("/", routes![login, post_login, logout, not_allowed])
     // Asynchronous application
     .mount("/", routes![poll_list, vote_for, post_vote_for])
     // Not logged in async routes
     .mount("/", routes![poll_list_not_logged, vote_for_not_logged])

     // Static below
     .mount("/public", StaticFiles::from(concat!(env!("CARGO_MANIFEST_DIR"), "/static")))
     .register(catchers![not_found])
     .launch();
}

