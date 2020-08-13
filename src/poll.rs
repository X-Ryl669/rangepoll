pub mod poll {
    extern crate chrono;
    extern crate glob; 

    use comrak::{markdown_to_html, ComrakOptions};

    use glob::glob;
    use chrono::{DateTime, Utc};
    use std::fs;
    use std::path::Path;

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

    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    pub struct PollDesc {
        name: String,
        desc: String,
        filepath: String,
        due_date: String,
        due_near: bool,
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

    pub fn parse_poll_file(path: &Path) -> Result<Poll, serde_yaml::Error> {
        let content = fs::read_to_string(path).expect("<FailureToReadFile>");
        //let poll: Poll = serde_yaml::from_reader(fs::File::open(path).expect("Unable to read file"))?;
        let mut poll: Poll = serde_yaml::from_str(&content)?;
        // If we don't have a description, let's fetch from markdown
        if poll.description.is_none() && poll.desc_markdown.is_none() {
            poll.desc = poll.name.clone();
        } else if poll.description.is_none() && poll.desc_markdown.is_some() {
            // Read the given file and convert to HTML here
            let rel_path_to_md_file = path.with_file_name(poll.desc_markdown.as_ref().unwrap().clone());
            let md_content = fs::read_to_string(rel_path_to_md_file).expect("<FailedToFindMarkdown>");
            poll.desc = markdown_to_html(&md_content, &ComrakOptions::default());
        } else {
            poll.desc = poll.description.as_ref().unwrap().clone();
        }
        poll.filepath = path.to_str().unwrap().to_string();
        return Ok(poll);
    //    let polls = YamlLoader::load_from_str(content).unwrap();
    //    let poll = &polls[0];

    //    return  Poll(name: poll["name"][0].as_str(), desc_markdown: fs::read_to_string(poll["desc"][0].as_str()))
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