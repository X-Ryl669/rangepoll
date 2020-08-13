pub mod voters {
    extern crate glob; 

    use glob::glob;
    use std::fs;
    use std::path::Path;

    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    pub struct Voter {
        pub name: String,
        pub presentation: String,
        pub password: String,
        pub admin: bool,
    }

   pub fn parse_voter_file(path: &Path) -> Result<Voter, serde_yaml::Error> {
        let content = fs::read_to_string(path).expect("<FailureToReadFile>");
        let voter: Voter = serde_yaml::from_str(&content)?;
        return Ok(voter);
    }

    pub fn get_voter_list() -> Result<Vec<Voter>, serde_yaml::Error> {
        let polls = glob(concat!(env!("CARGO_MANIFEST_DIR"), "/voters/*.yml")).expect("Failed to read glob pattern");
        let mut output = Vec::new();
        for entry in polls {
            match entry {
                Ok(path) => { 
                    println!("Found: {:?}", path.display()); 
                    match parse_voter_file(&path) {
                        Ok(poll) => output.push(poll),
                        Err(e) => println!("Failed parsing {:?} with error {:?}", path.display(), e),
                    }  
                },
                Err(e) => println!("Failed with error: {:?}", e),
            }
        }
        return Ok(output);
    }

    pub fn gen_template(dest: &str) {
        let voter = Voter { name: "Isaac".to_string(), 
                                presentation: "I'm one of the best physician".to_string(), 
                                password: "This is a very poor designed system".to_string(),
                                admin: glob("/voters/*.yml").expect("Failed to read glob pattern").count() == 0,
                            };
        let serial = serde_yaml::to_string(&voter);
        match serial {
            Ok(v) => fs::write(dest, v).expect("Failed writing"),
            Err(e) => println!("Failed to generate template {:?} with error: {:?}", dest, e),
        }
    }

}