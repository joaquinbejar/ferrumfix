use super::{Config, Configure, DecodeError};
use crate::dtf;
use crate::fields::{FieldDef, FieldLocation};
use crate::Dictionary;
use serde::{Deserialize, Serialize};
use std::borrow::{Borrow, Cow};
use std::collections::HashMap;

/// A read-only JSON FIX message as parsed by [`Decoder`].
#[derive(Debug, Copy, Clone)]
pub struct Message<'a> {
    internal: &'a MessageInternal<'a>,
}

/// A repeating group within a [`Message`].
#[derive(Debug, Copy, Clone)]
pub struct MessageGroup<'a> {
    message: &'a Message<'a>,
    entries: &'a [Component<'a>],
}

impl<'a> MessageGroup<'a> {
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn iter(&self) -> impl Iterator<Item = MessageGroupEntry<'a>> {
        self.entries
            .iter()
            .map(|component| MessageGroupEntry { component })
    }
}

/// A specific [`MessageGroup`] entry.
#[derive(Debug)]
pub struct MessageGroupEntry<'a> {
    component: &'a Component<'a>,
}

impl<'a> MessageGroupEntry<'a> {
    pub fn group<'b, T>(&'b self, _field_def: &FieldDef<'b, T>) -> Option<MessageGroup<'b>>
    where
        'b: 'a,
        T: dtf::DataField<'b>,
    {
        None
    }

    pub fn field_ref<'b, T>(
        &'b self,
        _field_def: &FieldDef<'b, T>,
    ) -> Option<Result<T, <T as dtf::DataField<'b>>::Error>>
    where
        'b: 'a,
        T: dtf::DataField<'b>,
    {
        unimplemented!()
    }

    pub fn field_raw(&self, _name: &str, _location: FieldLocation) -> Option<&str> {
        unimplemented!()
    }

    //type FieldsIter = FieldsIter<'a>;
    //type FieldsIterStdHeader = FieldsIter<'a>;
    //type FieldsIterBody = FieldsIter<'a>;

    /// Creates an [`Iterator`] over all FIX fields in `self`.
    pub fn iter_fields(&self) -> impl Iterator<Item = Cow<'a, str>> {
        // TODO
        std::iter::empty()
    }
}

impl<'a> Message<'a> {
    pub fn group<'b, T>(&'b self, _field_def: &FieldDef<'b, T>) -> Option<MessageGroup<'b>>
    where
        'b: 'a,
        T: dtf::DataField<'b>,
    {
        None
    }

    pub fn field_ref<'b, T>(
        &'b self,
        field_def: &FieldDef<'b, T>,
    ) -> Option<Result<T, <T as dtf::DataField<'b>>::Error>>
    where
        'b: 'a,
        T: dtf::DataField<'b>,
    {
        self.internal.field_ref(field_def)
    }

    pub fn field_raw(&self, name: &str, location: FieldLocation) -> Option<&str> {
        self.internal.field_raw(name, location)
    }

    //type FieldsIter = FieldsIter<'a>;
    //type FieldsIterStdHeader = FieldsIter<'a>;
    //type FieldsIterBody = FieldsIter<'a>;

    /// Creates an [`Iterator`] over all FIX fields in `self`.
    pub fn iter_fields(&self) -> impl Iterator<Item = Cow<'a, str>> {
        // TODO
        std::iter::empty()
    }
}

/// A codec for the JSON encoding type.
#[derive(Debug, Clone)]
pub struct Decoder<C = Config>
where
    C: Configure,
{
    dictionaries: HashMap<String, Dictionary>,
    message_builder: MessageInternal<'static>,
    config: C,
}

impl<C> Decoder<C>
where
    C: Configure,
{
    /// Creates a new codec. `dict` serves as a reference for data type inference
    /// of incoming messages' fields. `config` handles encoding details. See the
    /// [`Configure`] trait for more information.
    pub fn new(dict: Dictionary) -> Self {
        Self::with_config(dict, C::default())
    }

    /// Creates a new codec. `dict` serves as a reference for data type inference
    /// of incoming messages' fields. `config` handles encoding details. See the
    /// [`Configure`] trait for more information.
    pub fn with_config(dict: Dictionary, config: C) -> Self {
        let mut dictionaries = HashMap::new();
        dictionaries.insert(dict.get_version().to_string(), dict);
        Self {
            dictionaries,
            message_builder: MessageInternal::default(),
            config,
        }
    }

    /// Returns an immutable reference to the [`Configure`] implementor used by
    /// `self`.
    pub fn config(&self) -> &C {
        &self.config
    }

    /// Returns a mutable reference to the [`Configure`] implementor used by
    /// `self`.
    pub fn config_mut(&mut self) -> &mut C {
        &mut self.config
    }

    fn message_builder<'a>(&'a mut self) -> &'a mut MessageInternal<'a> {
        self.message_builder.clear();
        unsafe {
            std::mem::transmute::<&'a mut MessageInternal<'static>, &'a mut MessageInternal<'a>>(
                &mut self.message_builder,
            )
        }
    }

    pub fn decode<'a>(&'a mut self, data: &'a [u8]) -> Result<Message<'a>, DecodeError> {
        let mut deserilizer = serde_json::Deserializer::from_slice(data);
        let msg = self.message_builder();
        MessageInternal::deserialize_in_place(&mut deserilizer, msg)
            .map_err(|_| DecodeError::InvalidData)?;
        Ok(Message { internal: msg })
    }
}

type Component<'a> = HashMap<&'a str, FieldOrGroup<'a>>;

#[derive(Deserialize, Serialize, Debug, Clone)]
enum FieldOrGroup<'a> {
    Field {
        value: Cow<'a, str>,
    },
    Group {
        #[serde(borrow)]
        entries: Vec<Component<'a>>,
    },
}

#[derive(Deserialize, Serialize, Debug, Clone, Default)]
struct MessageInternal<'a> {
    #[serde(borrow, rename = "StandardHeader")]
    std_header: Component<'a>,
    #[serde(borrow, rename = "Body")]
    body: Component<'a>,
    #[serde(borrow, rename = "StandardTrailer")]
    std_trailer: Component<'a>,
}

impl<'a> std::ops::Drop for MessageInternal<'a> {
    fn drop(&mut self) {
        self.clear();
    }
}

impl<'a> MessageInternal<'a> {
    fn clear(&mut self) {
        self.std_header.clear();
        self.body.clear();
        self.std_trailer.clear();
    }

    pub fn field_ref<'b, T>(
        &'b self,
        field_def: &FieldDef<'b, T>,
    ) -> Option<Result<T, <T as dtf::DataField<'b>>::Error>>
    where
        'b: 'a,
        T: dtf::DataField<'b>,
    {
        self.field_raw(field_def.name(), field_def.location)
            .map(|s| T::deserialize(s.as_bytes()))
    }

    fn field_raw(&self, name: &str, location: FieldLocation) -> Option<&str> {
        match location {
            FieldLocation::Body => self.body.get(name).and_then(|field_or_group| {
                if let FieldOrGroup::Field { value } = field_or_group {
                    Some(value.borrow())
                } else {
                    None
                }
            }),
            _ => panic!(),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::AppVersion;

    const MESSAGE_SIMPLE: &str = include_str!("test_data/message_simple.json");

    const MESSAGE_WITHOUT_HEADER: &str = include_str!("test_data/message_without_header.json");

    fn dict_fix44() -> Dictionary {
        Dictionary::from_version(AppVersion::Fix44)
    }

    fn encoder_fix44() -> Decoder<impl Configure> {
        Decoder::with_config(dict_fix44(), Config::default())
    }

    #[test]
    fn message_without_header() {
        let mut encoder = encoder_fix44();
        let result = encoder.decode(&mut MESSAGE_WITHOUT_HEADER.as_bytes());
        match result {
            Err(DecodeError::Schema) => (),
            _ => panic!(),
        };
    }

    #[test]
    fn simple_message() {
        let mut encoder = encoder_fix44();
        let result = encoder.decode(&mut MESSAGE_SIMPLE.as_bytes());
        assert!(result.is_ok());
    }

    #[test]
    fn invalid_json() {
        let mut encoder = encoder_fix44();
        let result = encoder.decode(&mut "this is invalid JSON".as_bytes());
        match result {
            Err(DecodeError::Syntax) => (),
            _ => panic!(),
        };
    }
}