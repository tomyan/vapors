//! Components vapors ships, written in `.wisp` and compiled by
//! `build.rs`: each file under `src` becomes a module here
//! (`combobox.wisp` is [`combobox::Combobox`]). They depend on the tree
//! only through `vapors-dom` and on `vapors-signals`, so any `Dom` mounts
//! them (the layers check holds this). An app uses one by its path:
//! `<vapors_components::combobox::Combobox ... />`.

include!(concat!(env!("OUT_DIR"), "/wisp.rs"));
