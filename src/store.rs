// Copyright 2021 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

use serde_json::Value as JsonValue;
use std::{
    collections::HashMap,
    fs::{create_dir_all, read, File},
    io::Write,
    path::PathBuf,
};
use dirs::public_dir;

type SerializeFn =
    fn(&HashMap<String, JsonValue>) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>>;
type DeserializeFn =
    fn(&[u8]) -> Result<HashMap<String, JsonValue>, Box<dyn std::error::Error + Send + Sync>>;

fn default_serialize(
    cache: &HashMap<String, JsonValue>,
) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
    Ok(serde_json::to_vec(&cache)?)
}

fn default_deserialize(
    bytes: &[u8],
) -> Result<HashMap<String, JsonValue>, Box<dyn std::error::Error + Send + Sync>> {
    serde_json::from_slice(bytes).map_err(Into::into)
}

/// Builds a [`Store`]
pub struct StoreBuilder {
    path: PathBuf,
    defaults: Option<HashMap<String, JsonValue>>,
    cache: HashMap<String, JsonValue>,
    serialize: SerializeFn,
    deserialize: DeserializeFn,
}

impl StoreBuilder {
    /// Creates a new [`StoreBuilder`].
    ///
    /// # Examples
    /// ```
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// use tauri_plugin_store::StoreBuilder;
    ///
    /// let builder = StoreBuilder::new("store.bin".parse()?);
    ///
    /// # Ok(())
    /// # }
    /// ```
    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            defaults: None,
            cache: Default::default(),
            serialize: default_serialize,
            deserialize: default_deserialize,
        }
    }

    /// Inserts a default key-value pair.
    ///
    /// # Examples
    /// ```
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// use tauri_plugin_store::StoreBuilder;
    /// use std::collections::HashMap;
    ///
    /// let mut defaults = HashMap::new();
    ///
    /// defaults.insert("foo".to_string(), "bar".into());
    ///
    /// let builder = StoreBuilder::new("store.bin".parse()?)
    ///   .defaults(defaults);
    ///
    /// # Ok(())
    /// # }
    pub fn defaults(mut self, defaults: HashMap<String, JsonValue>) -> Self {
        self.cache = defaults.clone();
        self.defaults = Some(defaults);
        self
    }

    /// Inserts multiple key-value pairs.
    ///
    /// # Examples
    /// ```
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// use tauri_plugin_store::StoreBuilder;
    ///
    /// let builder = StoreBuilder::new("store.bin".parse()?)
    ///   .default("foo".to_string(), "bar".into());
    ///
    /// # Ok(())
    /// # }
    pub fn default(mut self, key: String, value: JsonValue) -> Self {
        self.cache.insert(key.clone(), value.clone());
        self.defaults
            .get_or_insert(HashMap::new())
            .insert(key, value);
        self
    }

    /// Defines a custom serialization function.
    ///
    /// # Examples
    /// ```
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// use tauri_plugin_store::StoreBuilder;
    ///
    /// let builder = StoreBuilder::new("store.json".parse()?)
    ///   .serialize(|cache| serde_json::to_vec(&cache).map_err(Into::into));
    ///
    /// # Ok(())
    /// # }
    pub fn serialize(mut self, serialize: SerializeFn) -> Self {
        self.serialize = serialize;
        self
    }

    /// Defines a custom deserialization function
    ///
    /// # Examples
    /// ```
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// use tauri_plugin_store::StoreBuilder;
    ///
    /// let builder = StoreBuilder::new("store.json".parse()?)
    ///   .deserialize(|bytes| serde_json::from_slice(&bytes).map_err(Into::into));
    ///
    /// # Ok(())
    /// # }
    pub fn deserialize(mut self, deserialize: DeserializeFn) -> Self {
        self.deserialize = deserialize;
        self
    }

    /// Builds the [`Store`].
    ///
    /// # Examples
    /// ```
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// use tauri_plugin_store::StoreBuilder;
    ///
    /// let store = StoreBuilder::new("store.bin".parse()?).build();
    ///
    /// # Ok(())
    /// # }
    pub fn build(self) -> Store {
        Store {
            path: self.path,
            defaults: self.defaults,
            cache: self.cache,
            serialize: self.serialize,
            deserialize: self.deserialize,
        }
    }
}

#[derive(Clone)]
pub struct Store {
    pub(crate) path: PathBuf,
    defaults: Option<HashMap<String, JsonValue>>,
    cache: HashMap<String, JsonValue>,
    serialize: SerializeFn,
    deserialize: DeserializeFn,
}

impl Store {
    /// Update the store from the on-disk state
    pub fn load(&mut self) -> Result<(), String> {
        let app_dir = public_dir().expect("failed to resolve public dir");
        let store_path = app_dir.join(&self.path);

        let bytes = read(store_path).map_err(|e| e.to_string())?;

        self.cache
            .extend((self.deserialize)(&bytes).map_err(|e| e.to_string())?);

        Ok(())
    }

    /// Saves the store to disk
    pub fn save(&self) -> Result<(), String> {
        let app_dir = public_dir().expect("failed to resolve public dir");

        let store_path = app_dir.join(&self.path);

        create_dir_all(store_path.parent().expect("invalid store path")).map_err(|e| e.to_string())?;

        let bytes = (self.serialize)(&self.cache).map_err(|e| e.to_string())?;
        let mut f = File::create(&store_path).map_err(|e| e.to_string())?;
        f.write_all(&bytes).map_err(|e| e.to_string())?;

        Ok(())
    }

    pub fn insert(&mut self, key: String, value: JsonValue) -> Result<(), String> {
        self.cache.insert(key.clone(), value.clone());

        Ok(())
    }

    pub fn get(&self, key: impl AsRef<str>) -> Option<&JsonValue> {
        self.cache.get(key.as_ref())
    }

    pub fn has(&self, key: impl AsRef<str>) -> bool {
        self.cache.contains_key(key.as_ref())
    }

    pub fn delete(&mut self, key: impl AsRef<str>) -> Result<bool, String> {
        let flag = self.cache.remove(key.as_ref()).is_some();
        Ok(flag)
    }

    pub fn clear(&mut self) -> Result<(), String> {
        let keys: Vec<String> = self.cache.keys().cloned().collect();
        self.cache.clear();
        Ok(())
    }

    pub fn reset(&mut self) -> Result<(), String> {
        let has_defaults = self.defaults.is_some();

        if has_defaults {
            if let Some(defaults) = &self.defaults {
                self.cache = defaults.clone();
            }
            Ok(())
        } else {
            self.clear()
        }
    }

    pub fn keys(&self) -> impl Iterator<Item = &String> {
        self.cache.keys()
    }

    pub fn values(&self) -> impl Iterator<Item = &JsonValue> {
        self.cache.values()
    }

    pub fn entries(&self) -> impl Iterator<Item = (&String, &JsonValue)> {
        self.cache.iter()
    }

    pub fn len(&self) -> usize {
        self.cache.len()
    }

    pub fn is_empty(&self) -> bool {
        self.cache.is_empty()
    }
}

impl std::fmt::Debug for Store {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Store")
            .field("path", &self.path)
            .field("defaults", &self.defaults)
            .field("cache", &self.cache)
            .finish()
    }
}
