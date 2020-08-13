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

    pub const DUE_FORMAT: &'static str = "%Y-%m-%d";

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
        name: String,
        desc: String,
        vote: Vec<usize>,
        voter: Vec<String>,
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
        due_date: DateTime<Utc>,
        choices: Vec<Choice>,
    }

    // This is for the parsed poll from a file
    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    pub struct ParsedPoll {
        name: String,
        desc: String,
        filepath: String,
        allowed_participant: Vec<String>,
        due_date: String,
        due_near: bool,
        choices: Vec<ParsedChoice>,
    }

    impl ParsedPoll {
        fn new(poll: &Poll) -> ParsedPoll {
            ParsedPoll { 
                name: poll.name.clone(), 
                desc: poll.desc.clone(), 
                filepath: poll.filepath.clone(), 
                allowed_participant: poll.allowed_participant.clone(), 
                due_date: format!("{}", poll.due_date.format(DUE_FORMAT)),
                due_near: false,
                choices: vec![],
            }
        }
    }

    // This is for the poll list output
    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    pub struct PollDesc {
        name: String,
        desc: String,
        filepath: String,
        due_date: String,
        due_near: bool,
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

    pub fn find_poll_desc(name: String) -> Result<Poll, NoFileOrYAMLParsingError> {
        // Get all poll and find the one with the good file
        let polls = get_poll_list()?;
        for entry in polls {
            match Path::new(&entry.filepath).file_stem() {
                Some(n) => {
                    if name.as_str() == n {
                        return Ok(entry);
                    }
                },
                None => {},
            }
        }
        return Err(NoFileOrYAMLParsingError::from(std::io::Error::new(std::io::ErrorKind::NotFound, name + " not found")));
    }

    pub fn get_poll_desc(name: String) -> Result<ParsedPoll, NoFileOrYAMLParsingError> {
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
            let close_date = poll.due_date.signed_duration_since(Utc::now()) < chrono::Duration::days(1);
            output.push(PollDesc { name: poll.name.clone(), desc: poll.desc.clone(), filepath: filepath, due_date: format!("{}", poll.due_date.format(DUE_FORMAT)), due_near: close_date });
        }
        return Ok(output);
    }

    pub fn gen_template(dest: &str) {
        let mut choices = Vec::new();
        choices.push(Choice { name:"pear".to_string(), desc: "".to_string(), description: Some("A *pear* is good".to_string()), desc_markdown: None, vote: vec![3, 4], voter: vec!["John".to_string(), "Bob".to_string()] });
        choices.push(Choice { name:"apple".to_string(), desc: "".to_string(), description: Some("An *apple* a day...".to_string()), desc_markdown: None, vote: vec![5, 2], voter: vec!["John".to_string(), "Bob".to_string()] });

        let poll = Poll {   name:"Best fruit".to_string(),
                            filepath: "".to_string(),
                            desc: "".to_string(),
                            description: Some("Choose your best fruit".to_string()),
                            desc_markdown: None, 
                            allowed_participant: vec!["John".to_string(), "Bob".to_string(), "Isaac".to_string()],
                            due_date: Utc::now(),
                            choices: choices,
                        };
        let serial = serde_yaml::to_string(&poll);
        match serial {
            Ok(v) => fs::write(dest, v).expect("Failed writing"),
            Err(e) => println!("Failed to generate template {:?} with error: {:?}", dest, e),
        }
    }
}