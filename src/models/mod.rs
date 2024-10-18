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
        T: DeserializeOwned, // Stellt sicher, dass der Typ T deserialisiert werden kann
        P: AsRef<Path>,      // Der Pfad wird als Referenz auf einen Pfad übergeben
    {
        // Öffne die Datei im Lese-Modus mit Puffer.
        let file = File::open(path)?;
        let reader = BufReader::new(file);

        // Lese den JSON-Inhalt der Datei als Instanz des Typs T.
        let value = serde_json::from_reader(reader)?;

        // Rückgabe des deserialisierten Wertes.
        Ok(value)
    }
    #[cfg(test)]
    pub fn read_yaml_from_file<T, P>(path: P) -> Result<T, Box<dyn std::error::Error>>
    where
        T: DeserializeOwned, // Stellt sicher, dass der Typ T deserialisiert werden kann
        P: AsRef<Path>,      // Der Pfad wird als Referenz auf einen Pfad übergeben
    {
        // Öffne die Datei im Lese-Modus mit Puffer.
        let file = File::open(path)?;
        let reader = BufReader::new(file);

        // Lese den JSON-Inhalt der Datei als Instanz des Typs T.
        let value = serde_yaml::from_reader(reader)?;

        // Rückgabe des deserialisierten Wertes.
        Ok(value)
    }
}
