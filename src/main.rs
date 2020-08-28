#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use] extern crate rocket;
#[macro_use] extern crate serde_derive;
extern crate clap;

use std::path::{ Path, PathBuf };
use std::collections::HashMap;
use rocket::response::NamedFile;
use rocket_contrib::templates::Template;
use rocket::http::{ Cookies, Cookie, Status };
use rocket::response::{ Flash, Redirect };
use rocket::request::{ Form, LenientForm };
use rocket::config::{ Config, Environment };
use rocket::State;
use rocket::response::status::Custom;
use std::sync::Mutex;

// Let authentication be checked with Request Guard
use rocket::request::{ self, Request, FromRequest };
use clap::{ App, Arg };
use std::net::IpAddr;
use url::{ Url };


mod poll;
mod voters;
mod config;
mod rp_error;
mod admin;

struct GlobalConfig
{
    config : Mutex<config::Config>,
}

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
impl<'f> request::FromForm<'f> for poll::VotesForVoter {
    type Error = ();

    fn from_form(form_items: &mut request::FormItems<'f>, _: bool) -> Result<Self, ()> {
        let mut votes = poll::VotesForVoter {
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

#[derive(FromForm)]
struct UpdateVoter {
    new_voter_filename: String,
    new_voter_name: String,
    new_voter_email: String,
    new_voter_presentation: String,
    new_voter_password: String,
    new_voter_admin: bool,
}

#[derive(Serialize)]
struct LoginContext {
    site_name: &'static str,
    dest: &'static str
}

#[get("/user", rank=1)]
fn get_user_menu(voter: Voter, cfg: State<GlobalConfig>) -> Template {
    // Check if the user is an admin and if we're allowed to go to the admin page
    let allow_admin = match cfg.config.lock() {
        Ok(v) => v.enable_admin,
        Err(_) => false,
    };
    let mut ctx = HashMap::new();
    ctx.insert("name", voter.name);
    ctx.insert("admin", allow_admin.to_string());
    return Template::render("user_menu", ctx);
}
#[get("/user", rank=2)]
fn get_user_menu_not_logged() -> Custom<String> {
    Custom(Status::Unauthorized, "".to_string())
}

#[get("/admin", rank=1)]
fn get_admin(voter: Voter, cfg: State<GlobalConfig>) -> Result< Template, Custom<Template> > {
    // Check if the user is an admin and if we're allowed to go to the admin page
    let allow_admin = match cfg.config.lock() {
        Ok(v) => v.enable_admin,
        Err(_) => false,
    };
    if !allow_admin {
        let mut ctx = HashMap::new();
        ctx.insert("msg", "Admin page disabled in configuration");
        return Err(Custom(Status::MethodNotAllowed, Template::render("error/421", ctx)));
    }
    let admin = admin::get_admin(&voter.name);
    return Ok(Template::render("admin", admin));
}
#[get("/admin", rank=2)]
fn get_admin_not_logged() -> Custom<Template> {
    Custom(Status::Unauthorized, Template::render("forbidden", vec![ ("dest", "/") ].into_iter().collect::<HashMap<&str, &str>>()))
}
#[get("/update_voter/<action>/<filename>", rank=1)]
fn get_update_voter(voter: Voter, cfg: State<GlobalConfig>, action: String, filename: String) -> Result< Redirect, Custom<Template> > {
    // Check if the user is an admin and if we're allowed to go to the admin page
    let allow_admin = match cfg.config.lock() {
        Ok(v) => v.enable_admin,
        Err(_) => false,
    };
    if !allow_admin  {
        let mut ctx = HashMap::new();
        ctx.insert("msg", "Admin page disabled in configuration");
        return Err(Custom(Status::MethodNotAllowed, Template::render("error/421", ctx)));
    }
    match admin::update_voter(&voter.name, &action, &filename, None)
    {
        Ok(_) => { return Ok(Redirect::to("/admin")); },
        Err(_e) =>
        {
            let mut ctx = HashMap::new();
            ctx.insert("msg", "Action not allowed");
            return Err(Custom(Status::MethodNotAllowed, Template::render("error/421", ctx)));
        }
    }
}
#[get("/update_voter/<_param..>", rank=3)]
fn get_update_voter_not_logged(_param: PathBuf) -> Custom<Template> {
    Custom(Status::Unauthorized, Template::render("forbidden", vec![ ("dest", "/") ].into_iter().collect::<HashMap<&str, &str>>()))
}
#[post("/update_voter", data="<new_voter>")]
fn post_update_voter(voter: Voter, cfg: State<GlobalConfig>, new_voter: LenientForm<UpdateVoter>) -> Result< Redirect, Custom<Template> > {
    // Check if the user is an admin and if we're allowed to go to the admin page
    let allow_admin = match cfg.config.lock() {
        Ok(v) => v.enable_admin,
        Err(_) => false,
    };
    if !allow_admin  {
        let mut ctx = HashMap::new();
        ctx.insert("msg", "Admin page disabled in configuration");
        return Err(Custom(Status::MethodNotAllowed, Template::render("error/421", ctx)));
    }
    let v = voters::Voter {
        name: new_voter.new_voter_name.clone(),
        email: Some(new_voter.new_voter_email.clone()),
        presentation: new_voter.new_voter_presentation.clone(),
        password: new_voter.new_voter_password.clone(),
        admin: new_voter.new_voter_admin,
        filename: None,
    };
    match admin::update_voter(&voter.name, "update", &new_voter.new_voter_filename, Some(&v))
    {
        Ok(_) => { return Ok(Redirect::to("/admin")); },
        Err(_e) =>
        {
            let mut ctx = HashMap::new();
            ctx.insert("msg", "Action not allowed");
            return Err(Custom(Status::MethodNotAllowed, Template::render("error/421", ctx)));
        }
    }
}


#[get("/token/<token>")]
fn log_with_token(mut cookies: Cookies, token: String) -> Result< Redirect, Custom<Template> > {
    // Using JWT token here for authentication 
    let voter = match poll::validate_token(&token) {
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
fn login(cfg: State<GlobalConfig>) -> Result<Template, Custom<Template> > {
    if cfg.config.lock().unwrap().disable_login {
        let mut ctx = HashMap::new();
        ctx.insert("msg", "Authentication disabled");
        return Err(Custom(Status::Unauthorized, Template::render("error/401", ctx)));
    } 
    let context = LoginContext { site_name: "range poll", dest: "/login" };
    Ok(Template::render("login", &context))
}

#[post("/login", data = "<user>")]
fn post_login(mut cookies: Cookies, user: LenientForm<User>) -> Result< Redirect, Custom<Template> > {
    let voters = match voters::get_voter_list() {
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
    let polls = match poll::get_poll_desc_list(&voter.name) {
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
    let mut ppoll = match poll::get_poll_desc(&poll) {
        Ok(v) => v,
        Err(_) => { 
            let mut ctx = HashMap::new();
            ctx.insert("msg", "No poll found on server");
            return Ok(Template::render("error/421", &ctx)); 
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
fn post_vote_for(poll: String, voter: Voter, form: Form<poll::VotesForVoter>) -> Result<Template, Flash<Redirect>> {
    // Don't trust the form submitter and only use the authentication token we have generated here for the voter's name.
    let vote = poll::VotesForVoter { name: voter.name.clone(), votes: form.votes.clone() };
    let mut ppoll = match poll::vote_for_poll(&poll, &vote) {
        Ok(v) => v,
        Err(e) => { return Err(Flash::error(Redirect::to(format!("/not_allowed/{}/{}", "/poll_list", poll)), format!("{:?}", e))); },
    };

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
    let mut pollr = match poll::get_poll_result(dest.as_str(), voter.name.clone()) {
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


// Temporary stuff below
#[get("/public/<file..>")]
fn static_files(file: PathBuf) -> Option<NamedFile> {
    NamedFile::open(Path::new("static/").join(file)).ok()
}


fn main() {
    // Start main backend
    let mut port;
    let mut host;
    let scheme;
    let cfg = GlobalConfig { config: Mutex::new(config::Config::new()) };

    let cmd_args = App::new("rangepoll")
                        .version("0.1.0")
                        .author("Cyril R. <boite.pour.spam@gmail.com>")
                        .about("A voting server to run poll with weighted choices and collecting results")
                        .arg(Arg::with_name("port")
                               .short("p")
                               .long("port")
                               .value_name("port number")
                               .help("Sets the webserver port to listen to")
                               .takes_value(true))
                        .arg(Arg::with_name("host").short("a").long("host").value_name("hostname").help("Specify the webserver hostname").default_value("localhost").takes_value(true))
                        .arg(Arg::with_name("poll").short("g").long("gen-template").value_name("FILE").help("Generate a template poll YAML file and save to template.yaml (recommanded: polls/example.yml)").takes_value(true))
                        .arg(Arg::with_name("voter").short("v").long("gen-voter").value_name("FILE").help("Generate a template voter YAML file and save to voter.yaml (recommanded: voters/voter.yml)").takes_value(true))
                        .arg(Arg::with_name("token").short("t").long("gen-token").value_name("poll name").help("Generate tokens for the given poll's voters so it can be distributed by email for example").takes_value(true))
                        .arg(Arg::with_name("config").short("c").long("config").value_name("FILE").help("Specify the configuration file to use").default_value("config.yml").takes_value(true))
                        .get_matches();
    
    // Deal with optional config path
    {
        let mut config = cfg.config.lock().unwrap();
        *config = match config::get_config(cmd_args.value_of("config")) {
            Ok(c) => c,
            Err(e) => { 
                eprintln!("Error while parsing config: {}", e); 
                eprintln!("Saved default config to: {}", config::save_config(None, cmd_args.value_of("config")).unwrap_or_default());
                return; 
            }
        };
        let host_url = match Url::parse(config.base_url.as_str()) {
            Ok(u) => u,
            Err(e) => { eprintln!("Invalid base URL in config: {}", e); return; }
        };

        host = host_url.host_str().unwrap_or("localhost").to_string();
        port = host_url.port().unwrap_or(80);
        scheme = host_url.scheme().to_string();
    }

    if let Some(o) = cmd_args.value_of("port") {
        port = o.parse().unwrap_or(8000);
    }
    if let Some(o) = cmd_args.value_of("host") {
        host = o.to_string();
    }
    {
        let mut config = cfg.config.lock().unwrap();
        config.base_url = format!("{}://{}:{}", scheme, host, port).to_string();
    }


    if let Some(o) = cmd_args.value_of("poll") {
        poll::gen_template(o);
        println!("Generated template poll file to {:?}", o);
        return;
    }
    if let Some(o) = cmd_args.value_of("voter") {
        voters::gen_template(o);
        println!("Generated template voter file to {:?}", o);
        return;
    }
    if let Some(o) = cmd_args.value_of("token") {
        let tokens = match poll::gen_voters_token(o) {
            Ok(v) => v,
            Err(e) => { eprintln!("Error: {}", e); return; }
        };

        let max_voter_name = tokens.iter().map(|x| x.voter.len()).max().unwrap();

        println!("{:width$}    Token", "Voter", width = max_voter_name);
        for token in tokens {
            println!("{:width$}    {}/token/{}", token.voter, cfg.config.lock().unwrap().base_url, token.token, width = max_voter_name);
        }
        return;
    }

    let host_interface = host.parse::<IpAddr>().unwrap_or("0.0.0.0".parse::<IpAddr>().unwrap());

    println!("Configuration used: {:?}", cfg.config.lock().unwrap());

    let config = Config::build(Environment::Staging)
                        .address(host_interface.to_string())
                        .port(port)
                        .finalize().expect("Error building webserver config");

    let r = rocket::custom(config);

//  Enable this if you prefer to use a rocket.toml file for configuration
//    let r = rocket::ignite();

    r.attach(Template::fairing())
     .manage(cfg)
     .mount("/", routes![index])
     // Ajax below
     // Login or logout
     .mount("/", routes![login, post_login, logout, not_allowed, log_with_token])
     // Asynchronous application
     .mount("/", routes![poll_list, vote_for, post_vote_for, vote_results, menu, 
                         get_user_menu, get_admin, get_update_voter, post_update_voter])
     // Not logged in async routes
     .mount("/", routes![poll_list_not_logged, vote_for_not_logged, vote_results_not_logged, menu_not_logged, 
                         get_user_menu_not_logged, get_admin_not_logged, get_update_voter_not_logged])

     // Static below
     .mount("/", routes![static_files])
     .register(catchers![not_found])
     .launch();
}

