//! Shader sources. Built into the game; in development builds they are read again from
//! the source folder, so a shader edited while the game runs reloads with the resources.

use std::borrow::Cow;

macro_rules! shader {
    ($name:literal) => {
        ($name, include_str!(concat!("shaders/", $name)))
    };
}

const BUILT_IN: [(&str, &str); 4] = [
    shader!("block.wgsl"),
    shader!("outline.wgsl"),
    shader!("post.wgsl"),
    shader!("ui.wgsl"),
];

pub fn module(device: &wgpu::Device, name: &str) -> wgpu::ShaderModule {
    device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some(name),
        source: wgpu::ShaderSource::Wgsl(source(name)),
    })
}

fn source(name: &str) -> Cow<'static, str> {
    #[cfg(debug_assertions)]
    {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("src/render/shaders")
            .join(name);
        if let Ok(text) = std::fs::read_to_string(path) {
            return Cow::Owned(text);
        }
    }
    let built_in = BUILT_IN.iter().find(|(n, _)| *n == name);
    Cow::Borrowed(built_in.map_or("", |(_, text)| text))
}

#[cfg(test)]
mod tests {
    #[test]
    fn every_shader_is_built_in() {
        for name in ["block.wgsl", "outline.wgsl", "post.wgsl", "ui.wgsl"] {
            assert!(!super::source(name).is_empty(), "{name}");
        }
    }
}
