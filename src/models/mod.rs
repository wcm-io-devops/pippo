pub mod auth;
pub mod config;
pub mod domain;
pub mod environment;
pub mod execution;
pub mod log;
pub mod pipeline;
pub mod program;
pub mod variables;

#[cfg(test)]
mod tests {
    use serde::de::DeserializeOwned;
    use std::fs::File;
    use std::io::BufReader;
    use std::path::Path;

    #[cfg(test)]
    pub fn read_json_from_file<T, P>(path: P) -> Result<T, Box<dyn std::error::Error>>
    where
        T: DeserializeOwned, // Verifies,that Type T can be deserialized
        P: AsRef<Path>,      // reference to path
    {
        // opens teh file in read-mode with buffer
        let file = File::open(path)?;
        let reader = BufReader::new(file);

        //  read the json content int instance of type T
        let value = serde_json::from_reader(reader)?;

        // return deserialized object.
        Ok(value)
    }
    #[cfg(test)]
    pub fn read_yaml_from_file<T, P>(path: P) -> Result<T, Box<dyn std::error::Error>>
    where
        T: DeserializeOwned, // Verifies,that Type T can be deserialized
        P: AsRef<Path>,      // reference to path
    {
        // opens teh file in read-mode with buffer
        let file = File::open(path)?;
        let reader = BufReader::new(file);

        //  read the json content int instance of type T
        let value = serde_yaml::from_reader(reader)?;

        // return deserialized object.
        Ok(value)
    }
}
