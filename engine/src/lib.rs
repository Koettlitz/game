use bevy::ecs::component::Component;

pub mod animation;
pub mod asset;
pub mod overworld;
pub mod progress;

#[derive(Component, Debug)]
pub struct Id(pub String);
