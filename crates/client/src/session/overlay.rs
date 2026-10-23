use super::Session;
use crate::debug_overlay::DebugView;
use crate::stage::Stage;

impl Session {
    pub(super) fn debug_lines(&self) -> Vec<String> {
        if !self.debug.visible {
            return Vec::new();
        }
        let camera = self.camera();
        let game = match &self.stage {
            Stage::Playing(world) => Some(&world.game),
            _ => None,
        };
        let adapter = self.gpu.adapter.get_info();
        self.debug.lines(&DebugView {
            content: &self.content,
            language: &self.applied.interface.language,
            camera: &camera,
            game,
            adapter: &adapter,
            render_distance: self.applied.graphics.render_distance,
        })
    }
}
