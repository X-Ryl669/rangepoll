
extern crate chrono;
extern crate glob; 

use comrak::{markdown_to_html, ComrakOptions};

use glob::glob;
use chrono::{DateTime, Utc, Timelike};
use std::fs;
use std::path::Path;
use std::collections::{ HashMap, HashSet };
use array2d::Array2D;
use std::iter::FromIterator;
use jsonwebtoken::{ encode, Algorithm, Header, EncodingKey, decode, DecodingKey, Validation };
use crate::rp_error::RPError;

pub const DEADLINE_FORMAT: &'static str = "%Y-%m-%d";

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct Choice {
    name: String,
    #[serde(skip)]
    desc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    desc_markdown: Option<String>,
    vote: Vec<usize>,
    voter: Vec<String>,
}

// This is the parsed choice from a file
#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct ParsedChoice {
    pub name: String,
    pub desc: String,
    pub vote: Vec<usize>,
    pub voter: Vec<String>,
}
impl ParsedChoice {
    fn new(choice: Choice, path: &Path) -> Result<ParsedChoice, RPError> {
        Ok(ParsedChoice { 
            name: choice.name.clone(),
            desc: build_desc(&choice.description, &choice.desc_markdown, path)?,
            vote: choice.vote.clone(),
            voter: choice.voter.clone(),
        })
    }
}


#[derive(Debug, PartialEq, Serialize, Deserialize, Copy, Clone)]
pub enum VotingAlgorithm {
    // This is the max of sum vote, that is the choice with the maximum total number of points wins 
    #[serde(rename = "max")]
    Max,
    // Used for simple Yes/No polling, the choice with the highest number of Yes wins 
    #[serde(rename = "binary")]
    Binary,
    // This is similar to max vote, that is the choice with the maximum total number of points wins but choice can't have the same number of point (obviously limited to 5 choices maximum)
    #[serde(rename = "bordat")]
    Bordat,
    // This is similar to mean consensus vote, that is each choice is compared to each other choice individually and the last winner wins the vote           
    #[serde(rename = "condorcet")]
    Condorcet,
    // This is similar to only select the best choice vote, that is only the preferred choice is kept for each voter regardless of the other choice, and the choice with the most voters wins      
    #[serde(rename = "first-choice")]
    FirstChoice,
    // This is similar to the French voting system, that is the choice counted per voters and vote, and only the 2 best choice are kept, then the other choice value are dispatched to compute the statistics and select the highest score    
    #[serde(rename = "french-system")]
    FrenchSystem,
    // In this mode, the choice with the lowest acceptance is eliminated and the other vote with a lower value are transfered to the other choice, repeat until only one remains
    #[serde(rename = "successive-elimination")]
    SuccessiveElimination,        
}

impl Default for VotingAlgorithm {
    fn default() -> Self { VotingAlgorithm::Max }
}

#[derive(Debug)]
pub struct VotesForVoter {
    pub name: String,
    pub votes: HashMap<String, u32>,
}

/*
impl VotingAlgorithm {
    pub fn from_str(s: &str) -> Option<VotingAlgorithm> {
        match s.to_ascii_lowercase().as_str() {
            "bordat" => Some(VotingAlgorithm::Bordat),
            "condorcet" => Some(VotingAlgorithm::Condorcet),
            "first_choice" => Some(VotingAlgorithm::FirstChoice),
            "french_system" => Some(VotingAlgorithm::FrenchSystem),
            "successive_elimination" => Some(VotingAlgorithm::SuccessiveElimination),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match *self {
            VotingAlgorithm::Bordat => "bordat",
            VotingAlgorithm::Condorcet => "condorcet",
            VotingAlgorithm::FirstChoice => "first_choice",
            VotingAlgorithm::FrenchSystem => "french_system",
            VotingAlgorithm::SuccessiveElimination => "successive_elimination",
        }
    }
}
*/

#[derive(Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct PollOptions {
    // Allow to skip a choice (in a vote)
    #[serde(rename = "allow-missing-choice", default)]
    pub allow_missing_choice:       bool,
    // Allow to vote while the due date is passed
    #[serde(rename = "allow-late-vote", default)]
    pub allow_late_vote:            bool,
    // Only show the result if every voter has voted
    #[serde(rename = "show-only-complete-result", default)]
    pub show_only_complete_result:  bool,

}


#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct Poll {
    name: String,

    #[serde(skip)]
    filepath: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    filename: Option<String>,
    
    // The cruft below is to support either description or desc_markdown key in the poll
    // In case the former is used, it's copied to this invisible desc field
    #[serde(skip)]
    desc: String,
    // Only either one is expected to be in the YAML file
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    // Else, the markdown key is used as a file path that's read and then converted to HTML
    #[serde(skip_serializing_if = "Option::is_none")]
    desc_markdown: Option<String>,
    allowed_participant: Vec<String>,
    
    #[serde(with = "date_serde")]
    deadline_date: DateTime<Utc>,
    choices: Vec<Choice>,

    // Any of Bordat / Condorcet / etc. (see VotingAlgorithm)
    #[serde(default)]
    voting_algorithm: VotingAlgorithm,

    #[serde(skip_serializing_if = "Option::is_none")]
    options: Option<PollOptions>,
}

// This is for the parsed poll from a file
#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct ParsedPoll {
    pub name: String,
    pub desc: String,
    pub filepath: String,
    pub filename: String,
    pub allowed_participant: Vec<String>,
    pub deadline_date: String,
    pub deadline_near: bool,
    pub algorithm: VotingAlgorithm,
    pub missing_choice: bool,
    pub choices: Vec<ParsedChoice>,
    pub user: String,
}

impl ParsedPoll {
    fn new(poll: &Poll) -> ParsedPoll {
        ParsedPoll { 
            name: poll.name.clone(), 
            desc: poll.desc.clone(), 
            filepath: poll.filepath.clone(), 
            filename: Path::new(&poll.filepath).file_stem().unwrap().to_str().unwrap().to_string(), 
            allowed_participant: poll.allowed_participant.clone(), 
            deadline_date: format!("{}", poll.deadline_date.format(DEADLINE_FORMAT)),
            deadline_near: false,
            algorithm: poll.voting_algorithm,
            missing_choice: match &poll.options { Some(v) => v.allow_missing_choice, None => false },
            choices: vec![],
            user: "".to_string(),
        }
    }
}

// This is for the poll list output
#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct PollDesc {
    name: String,
    desc: String,
    filepath: String,
    deadline_date: String,
    deadline_near: bool,
    deadline_passed: bool, // If the vote is done
    complete: bool,
    options: PollOptions,
}

// This is the poll result
#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct PollResult {
    pub name: String,
    pub desc: String,
    pub user: String, // Only used for user feedback
    pub algorithm: String,
    pub deadline_date: String,
    pub voters: Vec<String>,
    pub votes: Vec<String>,
    pub score: Vec<f32>,
    pub score_max: f32,
}
impl PollResult {
    fn error(name: &str, err: &str) -> PollResult {
        return PollResult { 
                name: name.to_string(), 
                desc: err.to_string(),
                voters: Vec::new(),
                deadline_date: "".to_string(),
                user: "".to_string(),
                algorithm: "".to_string(),
                votes: Vec::new(),
                score: Vec::new(),
                score_max: 0f32,
            };
    }

    fn new(poll: &Poll) -> PollResult {
        let def_option = PollOptions { ..Default::default() };
        let opt = poll.options.as_ref().unwrap_or(&def_option);

        let mut votes = Vec::new();

        // We are going to build a 2D matrix here of (row: voter, col:choice, cell: vote) since different algorithm need 
        // different access (some requires row access, some prefer column access)
        let mut choices = HashSet::new();
        let mut voters  = HashSet::new();
        for choice in &poll.choices {
            // Store all possible choices
            choices.insert(choice.name.clone());
            // Store all actual voters
            for voter in &choice.voter {
                voters.insert(voter.clone());
            }
        }
        // Shadow the parameters here so we have an ordered vector here
        let choices = Vec::from_iter(choices);
        let voters = Vec::from_iter(voters);
        // Let's build a matrix here
        let mut vote_matrix = Array2D::filled_with(0, voters.len(), choices.len());
        // And fill it now
        for choice in &poll.choices {
            let col = choices.iter().position(|x| x == &choice.name).unwrap();
            for i in 0..choice.voter.len() {
                let row = voters.iter().position(|x| x == &choice.voter[i]).unwrap();
                vote_matrix[(row, col)] = choice.vote[i];
            }
        }

        let mut score_max = 5f32;

        // First pass, make sure we have completed the vote
        if opt.show_only_complete_result && vote_matrix.as_row_major().iter().position(|&x| x == 0).is_some() {
            return PollResult::error(&poll.name, "<h1>Poll not completed yet</h1>");
        }

        if     poll.voting_algorithm != VotingAlgorithm::Max 
            && poll.voting_algorithm != VotingAlgorithm::Condorcet 
            && poll.voting_algorithm != VotingAlgorithm::Binary {
            for (row, name) in vote_matrix.as_rows().iter().zip(voters.iter()) { 
                let l: HashSet<&usize> = HashSet::from_iter(row.iter());
                if l.len() != row.len() {
                    return PollResult::error(&poll.name, &format!("<h1>Invalid vote result with {:?} algorithm, same vote by {}</h1>", poll.voting_algorithm, name));
                }
            }
        }

        match poll.voting_algorithm {
            // This is the max of sum vote, that is the choice with the maximum total number of points wins 
            VotingAlgorithm::Max => {
                // Compute sum of columns here
                for (col, name) in vote_matrix.as_columns().iter().zip(choices.iter()) {
                    votes.push((name.clone(), col.iter().sum::<usize>() as f32 / voters.len() as f32));
                }
            },
            // Used for simple Yes/No polling, the choice with the highest number of Yes wins
            VotingAlgorithm::Binary => {
                let mut max_score = 0;
                for (col, name) in vote_matrix.as_columns().iter().zip(choices.iter()) {
                    let score = col.iter().sum::<usize>();
                    if score > max_score {
                        max_score = score;
                    }
                    votes.push((name.clone(), score as f32));
                }
                score_max = max_score as f32;
            }
            // Bordat is similar to max vote, but all votes are first normalized (worst choice get 1 point, less worst get 2 points and so on) before being summed
            VotingAlgorithm::Bordat => {
                // Normalize votes first
                let rows = vote_matrix.as_rows();
                for (pos, row) in rows.iter().enumerate() {
                    let mut vote_for_voter = Vec::from_iter(row.iter().zip(choices.iter()));
                    vote_for_voter.sort_by(|a, b| a.0.cmp(b.0));
                    let mut acc = 1;
                    for (_, choice) in vote_for_voter {
                        vote_matrix[(pos, choices.iter().position(|x| x == choice).unwrap())] = acc;
                        acc += 1;
                    }
                }

                // Compute sum of columns here
                for (col, name) in vote_matrix.as_columns().iter().zip(choices.iter()) {
                    votes.push((name.clone(), col.iter().sum::<usize>() as f32 / voters.len() as f32));
                }
            },
            // This is similar to mean consensus vote, that is each choice is compared to each other choice individually and the last winner wins the vote           
            VotingAlgorithm::Condorcet => {
                // We need to compute, for each choice A, if it wins the next choice B (winning is defined as having more voter in favor of this choice A than B)
                // If it wins, B is dropped and A is compared to next choice C and so on, else A is dropped and B is compared to C and so on.
                let cols = vote_matrix.as_columns();

                // Dumb implementation in O(N^2) here, sorry, but it's easier and the number of choice will be limited anyway
                let min_score = voters.len() / 2; 
                for (col, name) in cols.iter().zip(choices.iter()) {
                    let mut score_duel = 0;
                    for other_col in &cols {
                        let score = col.iter().zip(other_col.iter()).map(|(a, b)| if a > b { 1 } else { 0 }).sum::<usize>();
                        if score > min_score { 
                            score_duel += 1; 
                        }
                    }
                    votes.push((name.clone(), score_duel as f32));
                }
                
                score_max = choices.len() as f32;
            },
            // This is similar to only select the best choice vote, that is only the preferred choice is kept for each voter regardless of the other choice, and the choice with the most voters wins      
            VotingAlgorithm::FirstChoice => {
                let mut vote_per_voter = vec![0;choices.len()];
                let rows = vote_matrix.as_rows();

                for row in rows.iter() {
                    let max_value = row.iter().max().unwrap();
                    let col_pos = row.iter().position(|x| x == max_value).unwrap();
                    vote_per_voter[col_pos] += 1;
                }
                for (vote, name) in vote_per_voter.iter().zip(choices.iter()) {
                    votes.push((name.clone(), *vote as f32));                    
                }

                score_max = choices.len() as f32;
            },
            // This is similar to the French voting system, that is the choice counted per voters and vote, and only the 2 best choice are kept, then the other choice value are dispatched to compute the statistics and select the highest score    
            VotingAlgorithm::FrenchSystem => {
                return PollResult::error(&poll.name, &format!("<h1>This {:?} algorithm isn't implemented yet</h1>", poll.voting_algorithm));
            },
            // In this mode, the choice with the lowest acceptance is eliminated and the other vote with a lower value are transfered to the other choice, repeat until only one remains
            VotingAlgorithm::SuccessiveElimination => {
                return PollResult::error(&poll.name, &format!("<h1>This {:?} algorithm isn't implemented yet</h1>", poll.voting_algorithm));
            }     
        }
        // Reverse sorting
        votes.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        PollResult { 
            name: poll.name.clone(),
            desc: poll.desc.clone(),
            voters: poll.allowed_participant.clone(),
            deadline_date: format!("{}", poll.deadline_date.format(DEADLINE_FORMAT)),
            user: "".to_string(),
            algorithm: format!("{:?}", poll.voting_algorithm),
            votes: votes.iter().map(|a| a.0.clone()).collect(),
            score: votes.iter().map(|a| a.1).collect(),
            score_max: score_max,
        }  
    }
}




// Our custom date formatter
mod date_serde {
    use chrono::{DateTime, Utc, TimeZone};
    use serde::{self, Deserialize, Serializer, Deserializer};

    pub const FORMAT: &'static str = "%Y-%m-%d %H:%M:%S";

    pub fn serialize<S>(date: &DateTime<Utc>, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer,
    {
        let s = format!("{}", date.format(FORMAT));
        serializer.serialize_str(&s)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<DateTime<Utc>, D::Error> where D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Utc.datetime_from_str(&s, FORMAT).map_err(serde::de::Error::custom)
    }
}

pub fn build_desc(description: &Option<String>, desc_markdown: &Option<String>, path: &Path) -> Result<String, RPError> {
    if description.is_none() && desc_markdown.is_none() {
        Ok(path.to_str().unwrap().to_string())
    } else if description.is_none() && desc_markdown.is_some() {
        // Read the given file and convert to HTML here
        let rel_path_to_md_file = path.with_file_name(desc_markdown.as_ref().unwrap().clone());
        let md_content = fs::read_to_string(rel_path_to_md_file)?;
        Ok(markdown_to_html(&md_content, &ComrakOptions::default()))
    } else {
        Ok(description.as_ref().unwrap().clone())
    }
}

pub fn parse_poll_file(path: &Path) -> Result<Poll, RPError> {
    let content = fs::read_to_string(path)?;
    //let poll: Poll = serde_yaml::from_reader(fs::File::open(path).expect("Unable to read file"))?;
    let mut poll: Poll = serde_yaml::from_str(&content)?;
    // If we don't have a description, let's fetch from markdown
    poll.desc = build_desc(&poll.description, &poll.desc_markdown, path)?;
    poll.filepath = path.to_str().unwrap().to_string();
    poll.filename = match path.file_stem() {
                        Some(path) => Some(path.to_str().unwrap().to_string()),
                        None => None,
                    };
    return Ok(poll);
}

pub fn find_poll_desc(name: &str) -> Result<Poll, RPError> {
    // Get all poll and find the one with the good file
    let polls = get_poll_list()?;
    for entry in polls {
        match Path::new(&entry.filepath).file_stem() {
            Some(n) => {
                if name == n {
                    return Ok(entry);
                }
            },
            None => {},
        }
    }
    return Err(RPError::from(std::io::Error::new(std::io::ErrorKind::NotFound, format!("{} not found", name))));
}

pub fn get_poll_desc(name: &str) -> Result<ParsedPoll, RPError> {
    let poll = find_poll_desc(name)?;
    // Copy all fields here
    let mut output = ParsedPoll::new(&poll);

    for entry in poll.choices {
        let path = poll.filepath.clone();
        output.choices.push(ParsedChoice::new(entry, Path::new(&path))?);
    }
    return Ok(output);
}

pub fn get_poll_list() -> Result<Vec<Poll>, serde_yaml::Error> {
    let polls = glob("./polls/*.yml").expect("Failed to read glob pattern");
    let mut output = Vec::new();
    for entry in polls {
        match entry {
            Ok(path) => { 
                println!("Found: {:?}", path.display()); 
                match parse_poll_file(&path) {
                    Ok(poll) => output.push(poll),
                    Err(e) => println!("Failed parsing {:?} with error {:?}", path.display(), e),
                }  
            },
            Err(e) => println!("Failed with error: {:?}", e),
        }
    }
    return Ok(output);
}

pub fn get_poll_desc_list(voter: &String) -> Result<Vec<PollDesc>, serde_yaml::Error> {
    let polls = get_poll_list()?;
    let mut output = Vec::new();
    for poll in polls {
        if !poll.allowed_participant.contains(voter) {
            continue;
        }
        let filepath = poll.filename.unwrap_or("".to_string());
        let close_date = poll.deadline_date.signed_duration_since(Utc::now()) < chrono::Duration::days(1);
        let done = poll.deadline_date.signed_duration_since(Utc::now()) < chrono::Duration::seconds(1);
        let opt = poll.options.unwrap_or_default();
        let complete = {
            let voters = poll.allowed_participant.len();
            let mut ret = true; 
            for choice in poll.choices {
                if choice.vote.len() < voters {
                    ret = false;
                    break;
                }
            }
            ret
        };
        output.push(PollDesc { name: poll.name.clone(), desc: poll.desc.clone(), filepath: filepath, deadline_date: format!("{}", poll.deadline_date.format(DEADLINE_FORMAT)), deadline_near: close_date, deadline_passed: done, options: opt, complete: complete });
    }
    return Ok(output);
}

pub fn compute_poll_result(poll: &Poll) -> Result<PollResult, RPError> {
    return Ok(PollResult::new(&poll));
}

pub fn get_poll_result(name: &str, voter_name: String) -> Result<PollResult, RPError> {
    let poll = find_poll_desc(name)?;
    if !poll.allowed_participant.contains(&voter_name) {
        return Err(RPError::from(std::io::Error::new(std::io::ErrorKind::PermissionDenied, format!("{} not allowed", voter_name))));
    }

    return compute_poll_result(&poll);
}

pub fn vote_for_poll(name: &str,  voters: &VotesForVoter) -> Result<PollResult, RPError> {
    let mut poll = find_poll_desc(name)?;
    if !poll.allowed_participant.contains(&voters.name) {
        return Err(RPError::from(std::io::Error::new(std::io::ErrorKind::PermissionDenied, format!("{} not allowed", voters.name))));
    }
    // Can we still accept this vote ?
    let late_vote = match &poll.options { Some(o) => o.allow_late_vote, None => false };
    if !late_vote && poll.deadline_date.signed_duration_since(Utc::now()) < chrono::Duration::seconds(1) {
        return Err(RPError::from(std::io::Error::new(std::io::ErrorKind::TimedOut, format!("{} deadline passed", name))));
    }
    let missing_choice = match &poll.options { Some(o) => o.allow_missing_choice, None => false };


    for choice in &mut poll.choices {
        let index = choice.voter.iter().position(|r| r == &voters.name);
        // Check if we have a vote for this choice
        let vote = voters.votes.get(&choice.name);
        if vote == None {
            if missing_choice {
                // No, we don't, let's skip this solution
                continue; 
            }
            else {
                return Err(RPError::from(std::io::Error::new(std::io::ErrorKind::InvalidInput, format!("{} invalid vote", name))));
            }
        }
        match index {
            Some(n) => choice.vote[n] = *vote.unwrap() as usize,
            None => { 
                choice.voter.push(voters.name.clone());
                choice.vote.push(*vote.unwrap() as usize);
            }
        }
    }
    // Then serialize the poll and save to file
    let serial = serde_yaml::to_string(&poll)?;
    fs::write(poll.filepath.clone(), serial)?;

    // Add or update the vote for the given voter
    return compute_poll_result(&poll);
}

pub fn gen_template(dest: &str) {
    let mut choices = Vec::new();
    choices.push(Choice { name:"pear".to_string(), desc: "".to_string(), description: Some("A pear is good".to_string()), desc_markdown: None, vote: vec![3, 4], voter: vec!["John".to_string(), "Bob".to_string()] });
    choices.push(Choice { name:"apple".to_string(), desc: "".to_string(), description: Some("An apple a day...".to_string()), desc_markdown: None, vote: vec![5, 2], voter: vec!["John".to_string(), "Bob".to_string()] });

    let poll = Poll {   name:"Best fruit".to_string(),
                        filepath: "".to_string(),
                        filename: None,
                        desc: "".to_string(),
                        description: Some("Choose your best fruit".to_string()),
                        desc_markdown: None, 
                        allowed_participant: vec!["John".to_string(), "Bob".to_string(), "Isaac".to_string()],
                        deadline_date: Utc::now(),
                        choices: choices,
                        voting_algorithm: VotingAlgorithm::Bordat,
                        options: None,
                    };
    let serial = serde_yaml::to_string(&poll);
    match serial {
        Ok(v) => fs::write(dest, v).expect("Failed writing"),
        Err(e) => println!("Failed to generate template {:?} with error: {:?}", dest, e),
    }
}

pub struct Token
{
    pub voter: String,
    pub token: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    sub: String,        // Used as the poll name
    company: String,    // Used as the voter's name
    #[serde(with = "jwt_numeric_date")]
    exp: DateTime<Utc>, // UTC timestamp
}

impl Claims {
    /// If a token should always be equal to its representation after serializing and deserializing
    /// again, this function must be used for construction. `DateTime` contains a microsecond field
    /// but JWT timestamps are defined as UNIX timestamps (seconds). This function normalizes the
    /// timestamps.
    pub fn new(sub: String, company: String, exp: DateTime<Utc>) -> Self {
        // normalize the timestamps by stripping of microseconds
        let exp = exp.date().and_hms_milli(exp.hour(), exp.minute(), exp.second(), 0);
        Self { sub, company, exp }
    }
}

mod jwt_numeric_date {
    //! Custom serialization of DateTime<Utc> to conform with the JWT spec (RFC 7519 section 2, "Numeric Date")
    use chrono::{DateTime, TimeZone, Utc};
    use serde::{self, Deserialize, Deserializer, Serializer};

    /// Serializes a DateTime<Utc> to a Unix timestamp (milliseconds since 1970/1/1T00:00:00T)
    pub fn serialize<S>(date: &DateTime<Utc>, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer,
    {
        let timestamp = date.timestamp();
        serializer.serialize_i64(timestamp)
    }

    /// Attempts to deserialize an i64 and use as a Unix timestamp
    pub fn deserialize<'de, D>(deserializer: D) -> Result<DateTime<Utc>, D::Error> where D: Deserializer<'de>,
    {
        Utc.timestamp_opt(i64::deserialize(deserializer)?, 0)
            .single() // If there are multiple or no valid DateTimes from timestamp, return None
            .ok_or_else(|| serde::de::Error::custom("invalid Unix timestamp value"))
    }
}

pub fn gen_voters_token(name: &str) -> Result<Vec<Token>, RPError> {
    let poll = find_poll_desc(name)?;
    let secret = match fs::read_to_string("secret.txt") {
        Ok(v) => v,
        Err(e) => { return Err(RPError::from(std::io::Error::new(std::io::ErrorKind::NotFound, format!("secret.txt not found: {}", e)))); }
    };

    let enc_key = EncodingKey::from_secret(secret.as_bytes());
    let mut output = Vec::new();
    for voter in poll.allowed_participant {
        let claim = Claims::new(name.to_string(), voter.clone(), poll.deadline_date + chrono::Duration::days(30));

        let token = Token {
            voter: voter.clone(),
            token: encode(&Header::default(), &claim, &enc_key).unwrap(),
        };
        output.push(token);
    }

    return Ok(output);
}

pub fn validate_token(token: &String) -> Result<(String, String), RPError> {
    let secret = match fs::read_to_string("secret.txt") {
        Ok(v) => v,
        Err(e) => { return Err(RPError::from(std::io::Error::new(std::io::ErrorKind::NotFound, format!("secret.txt not found: {}", e)))); }
    };
    let dec_key = &DecodingKey::from_secret(secret.as_bytes());
    let token_msg = match decode::<Claims>(token, &dec_key, &Validation::new(Algorithm::HS256)) {
        Ok(v) => v,
        Err(_) => { return Err(RPError::from(std::io::Error::new(std::io::ErrorKind::InvalidData, format!("{} invalid", token)))); }
    };
    let poll = find_poll_desc(&token_msg.claims.sub)?;
    if !poll.allowed_participant.contains(&token_msg.claims.company) {
        return Err(RPError::from(std::io::Error::new(std::io::ErrorKind::PermissionDenied, "Access denied")));
    }
    return Ok((token_msg.claims.sub.clone(), token_msg.claims.company.clone()));
}
