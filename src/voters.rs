extern crate glob; 

use glob::glob;
use std::fs;
use std::path::Path;

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct Voter {
    pub username: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fullname: Option<String>,
    pub presentation: String,
    pub password: String,
    
    pub admin: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filename: Option<String>,
}

pub fn parse_voter_file(path: &Path) -> Result<Voter, serde_yaml::Error> {
    let content = fs::read_to_string(path).expect("<FailureToReadFile>");
    let mut voter: Voter = serde_yaml::from_str(&content)?;
    voter.filename = Some(path.file_stem().unwrap().to_str().unwrap().to_string());
    return Ok(voter);
}

pub fn get_voter_list() -> Result<Vec<Voter>, serde_yaml::Error> {
    let voters = glob("./voters/*.yml").expect("Failed to read glob pattern");
    let mut output = Vec::new();
    for entry in voters {
        match entry {
            Ok(path) => { 
                println!("Found: {:?}", path.display()); 
                match parse_voter_file(&path) {
                    Ok(voter) => output.push(voter),
                    Err(e) => println!("Failed parsing {:?} with error {:?}", path.display(), e),
                }  
            },
            Err(e) => println!("Failed with error: {:?}", e),
        }
    }
    return Ok(output);
}

pub fn delete_voter(filestem: &str) -> bool {
    fs::remove_file(format!("voters/{}.yml", filestem)).is_ok() 
}

pub fn update_voter(filestem: &str, voter: &Voter) -> bool {
    let serial = serde_yaml::to_string(&voter);
    match serial {
        Ok(v) => { fs::write(format!("voters/{}.yml", filestem), v).expect("Failed writing"); true },
        Err(e) => { println!("Failed to save file {:?} with error: {:?}", filestem, e); false },
    }
}


pub fn gen_template(dest: &str) {
    let voter = Voter { 
                        username: "Isaac".to_string(), 
                        presentation: "I'm one of the best physician".to_string(), 
                        fullname: Some("Isaac Newton".to_string()),
                        email: Some("notinventedyet@newton.co.uk".to_string()),
                        password: "This is a very poor designed system".to_string(),
                        admin: glob("./voters/*.yml").expect("Failed to read glob pattern").count() == 0,
                        filename: None,
                    };
    let serial = serde_yaml::to_string(&voter);
    match serial {
        Ok(v) => fs::write(dest, v).expect("Failed writing"),
        Err(e) => println!("Failed to generate template {:?} with error: {:?}", dest, e),
    }
}
