use bevy::ecs::component::Component;

pub mod animation;
pub mod assets;
pub mod overworld;
pub mod progress;

#[derive(Component, Debug)]
pub struct Id(pub String);
