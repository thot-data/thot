use crate::{
    query::{AssetQuery, ContainerQuery},
    state,
};
use syre_core::db::SearchFilter;

impl SearchFilter<state::Container> for ContainerQuery {
    fn matches(&self, obj: &state::Container) -> bool {
        let state::DataResource::Ok(props) = &obj.properties() else {
            return false;
        };

        if let Some(s_name) = self.name.as_ref() {
            if s_name != &props.name {
                return false;
            }
        }

        if let Some(s_kind) = self.kind.as_ref() {
            if s_kind != &props.kind {
                return false;
            }
        }

        if !self.tags.iter().all(|tag| props.tags.contains(tag)) {
            return false;
        }

        for crate::query::Metadatum {
            key: s_key,
            value: s_val,
        } in self.metadata.iter()
        {
            let Some(f_val) = props.metadata.get(s_key) else {
                return false;
            };

            // only compare number values, not types
            if f_val.is_number() && s_val.is_number() {
                if f_val.as_f64() != s_val.as_f64() {
                    return false;
                }
            } else {
                if f_val != s_val {
                    return false;
                }
            }
        }

        // all search criteria matched
        true
    }
}

impl SearchFilter<state::Asset> for AssetQuery {
    fn matches(&self, obj: &state::Asset) -> bool {
        let asset = &obj.properties;
        let props = &asset.properties;

        if let Some(s_path) = self.path.as_ref() {
            if *s_path != asset.path {
                return false;
            }
        }

        if let Some(s_name) = self.name.as_ref() {
            if *s_name != props.name {
                return false;
            }
        }

        if let Some(s_kind) = self.kind.as_ref() {
            if *s_kind != props.kind {
                return false;
            }
        }

        if !self.tags.iter().all(|tag| props.tags.contains(tag)) {
            return false;
        }

        for crate::query::Metadatum {
            key: s_key,
            value: s_val,
        } in self.metadata.iter()
        {
            let Some(f_val) = props.metadata.get(s_key) else {
                return false;
            };

            // only compare number values, not types
            if f_val.is_number() && s_val.is_number() {
                if f_val.as_f64() != s_val.as_f64() {
                    return false;
                }
            } else {
                if f_val != s_val {
                    return false;
                }
            }
        }

        // all search criteria matched
        true
    }
}
