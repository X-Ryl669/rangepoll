pub mod poll {
    extern crate chrono;
    extern crate glob; 

    use comrak::{markdown_to_html, ComrakOptions};

    use glob::glob;
    use chrono::{DateTime, Utc};
    use std::fs;
    use std::path::Path;
    use std::error;
    use std::fmt;
    use std::collections::HashMap;

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
        fn new(choice: Choice, path: &Path) -> Result<ParsedChoice, NoFileOrYAMLParsingError> {
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
        pub deadline_date: String,
        pub voters: Vec<String>,
        pub votes: Vec<String>,
        pub score: Vec<f32>,
    }
    impl PollResult {
        fn new(poll: &Poll) -> PollResult {
            let def_option = PollOptions { ..Default::default() };
            let opt = poll.options.as_ref().unwrap_or(&def_option);

            let mut votes = Vec::new();
            // First pass, make sure we have completed the vote
            if opt.show_only_complete_result {
                for choice in &poll.choices {
                    if choice.voter.len() != poll.allowed_participant.len() {
                        return PollResult { 
                            name: poll.name.clone(), 
                            desc: "<h1>Poll not completed yet</h1>".to_string(),
                            voters: poll.allowed_participant.clone(),
                            deadline_date: format!("{}", poll.deadline_date.format(DEADLINE_FORMAT)),
                            user: "".to_string(),
                            votes: Vec::new(),
                            score: Vec::new(),
                        };
                    }
                }
            }

            match poll.voting_algorithm {
                VotingAlgorithm::Max => {
                    for choice in &poll.choices {
                        let mut sum: usize = 0; 
                        for vote in &choice.vote { 
                            sum = sum + vote 
                        }
                        let res: f32 = (sum as f32) / (choice.vote.len() as f32);
                        
                        votes.push((choice.name.clone(), res));
                    }
                },
                VotingAlgorithm::Bordat => {
                    for choice in &poll.choices {
                        let mut sum: usize = 0; 
                        let mut unique_vote = HashMap::new();
                        for vote in &choice.vote { 
                            sum = sum + vote;
                            if unique_vote.contains_key(vote) {
                                return PollResult { 
                                    name: poll.name.clone(), 
                                    desc: format!("<h1>Invalid vote result with {:?} algorithm, same vote {} for choice {}</h1>", poll.voting_algorithm, vote, choice.name),
                                    voters: poll.allowed_participant.clone(),
                                    deadline_date: format!("{}", poll.deadline_date.format(DEADLINE_FORMAT)),
                                    user: "".to_string(),
                                    votes: Vec::new(),
                                    score: Vec::new(),
                                };
                            }
                            unique_vote.insert(vote, choice.name.clone());
                        }
                        let res: f32 = (sum as f32) / (choice.vote.len() as f32);
                        
                        votes.push((choice.name.clone(), res));
                    }
                },
                // This is similar to mean consensus vote, that is each choice is compared to each other choice individually and the last winner wins the vote           
                VotingAlgorithm::Condorcet => {
                    // We need to compute, for each choice A, if it wins the next choice B (winning is defined as having more voter in favor of this choice A than B)
                    // If it wins, B is dropped and A is compared to next choice C and so on, else A is dropped and B is compared to C and so on.
                    let mut cur_win = 0;
                    for i in 0..poll.choices.len()-1 {
                        let A = &poll.choices[cur_win];
                        let B = &poll.choices[i+1];

                        // Compare between A and B
                        let mut wins_for_A = 0; 
                        let mut wins_for_B = 0; 
                        
                        if A.vote.len() != B.vote.len() {
                            return PollResult { 
                                    name: poll.name.clone(), 
                                    desc: format!("<h1>Invalid vote result with {:?} algorithm, missing vote for choice {}</h1>", poll.voting_algorithm, A.name),
                                    voters: poll.allowed_participant.clone(),
                                    deadline_date: format!("{}", poll.deadline_date.format(DEADLINE_FORMAT)),
                                    user: "".to_string(),
                                    votes: Vec::new(),
                                    score: Vec::new(),
                                };
                        }

                        for j in 0..A.vote.len() {
                            let v_A = A.vote[j];
                            let v_B = B.vote[j];
                            if v_A > v_B { 
                                wins_for_A += 1 
                            } else { 
                                wins_for_B += 1 
                            };
                        }

                        // Not sure what to do if there is an ex-aequo here,  
                        if wins_for_B > wins_for_A {
                            cur_win = i+1;
                        }
                    }
                    // The cur_win index is the winner, so let's save it now (sorry, we are not ranking the other here, since they are eliminated)
                    votes.push((poll.choices[cur_win].name.clone(), 5.0 as f32));
                },
                // This is similar to only select the best choice vote, that is only the preferred choice is kept for each voter regardless of the other choice, and the choice with the most voters wins      
                VotingAlgorithm::FirstChoice => {
                    let mut map : HashMap<String, (String, usize)> = HashMap::new();

                    for choice in &poll.choices {
                        // We have to create a map of voter to their best choice, but we have a structure of choice to voter
                        let mut i = 0;
                        // Find max first for each voter
                        for voter in &choice.voter {
                            match map.get(voter) {
                                Some(v) => { if v.1 < choice.vote[i] { map.insert(voter.to_string(), (choice.name.clone(), choice.vote[i])); } },
                                None    => { map.insert(voter.to_string(), (choice.name.clone(), choice.vote[i])); },
                            }
                            i += 1;
                        }
                    }
                    // Then compute the winner
                    let mut accumulator: HashMap<String, usize> = HashMap::new();
                    // Accumulate the choices counter
                    for voter in map.keys() {
                        let (choice,_) = map.get(voter).unwrap();
                        let mut c = match accumulator.get(choice) { Some(v) => *v, None => 0 };
                        c += 1; 
                        accumulator.insert(choice.to_string(), c);
                    }
                    // Then collect them in a vector
                    for choice in accumulator.keys() {
                        votes.push((choice.clone(), *accumulator.get(choice).unwrap() as f32));
                    }
                },
                // This is similar to the French voting system, that is the choice counted per voters and vote, and only the 2 best choice are kept, then the other choice value are dispatched to compute the statistics and select the highest score    
                VotingAlgorithm::FrenchSystem => {
                    return PollResult { 
                                    name: poll.name.clone(), 
                                    desc: format!("<h1>This {:?} algorithm isn't implemented yet</h1>", poll.voting_algorithm),
                                    voters: poll.allowed_participant.clone(),
                                    deadline_date: format!("{}", poll.deadline_date.format(DEADLINE_FORMAT)),
                                    user: "".to_string(),
                                    votes: Vec::new(),
                                    score: Vec::new(),
                                };
                },
                // In this mode, the choice with the lowest acceptance is eliminated and the other vote with a lower value are transfered to the other choice, repeat until only one remains
                VotingAlgorithm::SuccessiveElimination => {
                    return PollResult { 
                                    name: poll.name.clone(), 
                                    desc: format!("<h1>This {:?} algorithm isn't implemented yet</h1>", poll.voting_algorithm),
                                    voters: poll.allowed_participant.clone(),
                                    deadline_date: format!("{}", poll.deadline_date.format(DEADLINE_FORMAT)),
                                    user: "".to_string(),
                                    votes: Vec::new(),
                                    score: Vec::new(),
                                };

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
                votes: votes.iter().map(|a| a.0.clone()).collect(),
                score: votes.iter().map(|a| a.1).collect(),
            }  
        }
    }

    // The "no file or yaml error type"
    #[derive(Debug)]
    pub enum NoFileOrYAMLParsingError {
        IOError(std::io::Error),
        YAMLError(serde_yaml::Error),
    }
    impl fmt::Display for NoFileOrYAMLParsingError {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            match *self {
                NoFileOrYAMLParsingError::IOError(ref e) => e.fmt(f),
                // This is a wrapper, so defer to the underlying types' implementation of `fmt`.
                NoFileOrYAMLParsingError::YAMLError(ref e) => e.fmt(f),
            }
        }
    }

    impl error::Error for NoFileOrYAMLParsingError {
        fn source(&self) -> Option<&(dyn error::Error + 'static)> {
            match *self {
                NoFileOrYAMLParsingError::IOError(ref e) => Some(e),
                // The cause is the underlying implementation error type. Is implicitly
                // cast to the trait object `&error::Error`. This works because the
                // underlying type already implements the `Error` trait.
                NoFileOrYAMLParsingError::YAMLError(ref e) => Some(e),
            }
        }
    }

    // Implement the conversion from `serde_yaml::Error` to `NoFileOrYAMLParsingError`.
    // This will be automatically called by `?` if a `serde_yaml::Error`
    // needs to be converted into a `NoFileOrYAMLParsingError`.
    impl From<serde_yaml::Error> for NoFileOrYAMLParsingError {
        fn from(err: serde_yaml::Error) -> NoFileOrYAMLParsingError {
            NoFileOrYAMLParsingError::YAMLError(err)
        }
    }
    impl From<std::io::Error> for NoFileOrYAMLParsingError {
        fn from(err: std::io::Error) -> NoFileOrYAMLParsingError {
            NoFileOrYAMLParsingError::IOError(err)
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

    pub fn build_desc(description: &Option<String>, desc_markdown: &Option<String>, path: &Path) -> Result<String, NoFileOrYAMLParsingError> {
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

    pub fn parse_poll_file(path: &Path) -> Result<Poll, NoFileOrYAMLParsingError> {
        let content = fs::read_to_string(path)?;
        //let poll: Poll = serde_yaml::from_reader(fs::File::open(path).expect("Unable to read file"))?;
        let mut poll: Poll = serde_yaml::from_str(&content)?;
        // If we don't have a description, let's fetch from markdown
        poll.desc = build_desc(&poll.description, &poll.desc_markdown, path)?;
        poll.filepath = path.to_str().unwrap().to_string();
        return Ok(poll);
    }

    pub fn find_poll_desc(name: &str) -> Result<Poll, NoFileOrYAMLParsingError> {
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
        return Err(NoFileOrYAMLParsingError::from(std::io::Error::new(std::io::ErrorKind::NotFound, format!("{} not found", name))));
    }

    pub fn get_poll_desc(name: &str) -> Result<ParsedPoll, NoFileOrYAMLParsingError> {
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
        let polls = glob(concat!(env!("CARGO_MANIFEST_DIR"), "/polls/*.yml")).expect("Failed to read glob pattern");
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

    pub fn get_poll_desc_list() -> Result<Vec<PollDesc>, serde_yaml::Error> {
        let polls = get_poll_list()?;
        let mut output = Vec::new();
        for poll in polls {
            let filepath = match Path::new(&poll.filepath).file_stem() {
                Some(path) => path.to_str().unwrap().to_string(),
                None => "".to_string(),
            };
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

    pub fn compute_poll_result(poll: &Poll) -> Result<PollResult, NoFileOrYAMLParsingError> {
        return Ok(PollResult::new(&poll));
    }

    pub fn get_poll_result(name: &str, voter_name: String) -> Result<PollResult, NoFileOrYAMLParsingError> {
        let poll = find_poll_desc(name)?;
        if !poll.allowed_participant.contains(&voter_name) {
            return Err(NoFileOrYAMLParsingError::from(std::io::Error::new(std::io::ErrorKind::PermissionDenied, format!("{} not allowed", voter_name))));
        }

        return compute_poll_result(&poll);
    }

    pub fn vote_for_poll(name: &str,  voters: &VotesForVoter) -> Result<PollResult, NoFileOrYAMLParsingError> {
        let mut poll = find_poll_desc(name)?;
        if !poll.allowed_participant.contains(&voters.name) {
            return Err(NoFileOrYAMLParsingError::from(std::io::Error::new(std::io::ErrorKind::PermissionDenied, format!("{} not allowed", voters.name))));
        }
        // Can we still accept this vote ?
        let late_vote = match &poll.options { Some(o) => o.allow_late_vote, None => false };
        if !late_vote && poll.deadline_date.signed_duration_since(Utc::now()) < chrono::Duration::seconds(1) {
            return Err(NoFileOrYAMLParsingError::from(std::io::Error::new(std::io::ErrorKind::TimedOut, format!("{} deadline passed", name))));
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
                    return Err(NoFileOrYAMLParsingError::from(std::io::Error::new(std::io::ErrorKind::InvalidInput, format!("{} invalid vote", name))));
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
}