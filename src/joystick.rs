use sdl2::joystick::Joystick;
use serde::de;
use serde::de::{Deserializer, Visitor};
use serde::{Deserialize, Serialize, Serializer};
use std::fmt;

macro_rules! parse_hex {
    ($char:expr, $self:expr) => {
        $char
            .to_digit(16)
            .map(|x| x as u8)
            .ok_or(de::Error::invalid_value(
                de::Unexpected::Char($char),
                &$self,
            ))
    };
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct JoystickInfo {
    pub instance_id: i32,
    pub guid: JoystickGuid,
    // NOTE: index is the second unique identifier for guid when there are collisions, mostly 0
    pub index: usize,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct JoystickGuid([u8; 16]);

impl Serialize for JoystickGuid {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let hex_values: Vec<String> = self.0.iter().map(|x| format!("{:02x}", x)).collect();
        let s: String = hex_values.iter().flat_map(|s| s.chars()).collect();
        serializer.serialize_str(&s)
    }
}

impl<'de> Deserialize<'de> for JoystickGuid {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_str(JoystickGuidVisitor)
    }
}

struct JoystickGuidVisitor;

impl<'de> Visitor<'de> for JoystickGuidVisitor {
    type Value = JoystickGuid;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("hexadecimal string of 32 digits")
    }

    fn visit_str<E>(self, value: &str) -> Result<JoystickGuid, E>
    where
        E: de::Error,
    {
        if value.len() != 32 {
            return Err(de::Error::invalid_length(value.len(), &self));
        }

        let mut res: [u8; 16] = [0; 16];
        let mut it = value.chars();
        for i in 0..16 {
            let c1 = it.next().unwrap();
            let c2 = it.next().unwrap();

            res[i] = parse_hex!(c1, self)? * 16 + parse_hex!(c2, self)?;
        }

        Ok(JoystickGuid(res))
    }
}

impl JoystickInfo {
    pub fn new(joystick: &Joystick, index: usize) -> JoystickInfo {
        let instance_id = joystick.instance_id();
        let guid = JoystickGuid(joystick.guid().raw().data);

        JoystickInfo {
            instance_id,
            guid,
            index,
        }
    }
}
