// Copyright 2022 Andrew Conway.
// This file is part of ConcreteSTV.
// ConcreteSTV is free software: you can redistribute it and/or modify it under the terms of the GNU Affero General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.
// ConcreteSTV is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU Affero General Public License for more details.
// You should have received a copy of the GNU Affero General Public License along with ConcreteSTV.  If not, see <https://www.gnu.org/licenses/>.

use std::fmt;
use serde::{de, Serializer};

/// utility function for serde serializing a list of integers as a comma separated list.
/// Useful for using with actix get-parameter serialization.
pub fn serialize_stringified_usize_list<S:Serializer>(x: &Vec<usize>, s: S) -> Result<S::Ok, S::Error>
{
    s.serialize_str(&x.iter().map(|v|v.to_string()).collect::<Vec<_>>().join(","))
}

// reverse of serialize_stringified_usize_list
/// based on https://stackoverflow.com/questions/63844460/how-can-i-receive-multiple-query-params-with-the-same-name-in-actix-web
pub fn deserialize_stringified_usize_list<'de, D: de::Deserializer<'de>>(deserializer: D) -> Result<Vec<usize>, D::Error>
{
    struct StringVecVisitor;

    impl<'de> de::Visitor<'de> for StringVecVisitor {
        type Value = Vec<usize>;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("a string containing a list of unsigned integers")
        }

        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
        {
            let mut ids = Vec::new();
            if v.len()>0 {
                for id in v.split(",") {
                    let id = id.parse::<usize>().map_err(E::custom)?;
                    ids.push(id);
                }
            }
            Ok(ids)
        }
    }

    deserializer.deserialize_any(StringVecVisitor)
}
