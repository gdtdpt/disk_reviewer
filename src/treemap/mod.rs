pub mod types;
pub mod layout;
pub mod color;
pub mod renderer;

pub use types::TreemapNode;
pub use layout::layout_treemap;
pub use renderer::paint_treemap;

/// 用户与 Treemap 的交互动作
pub enum TreemapAction {
    /// 单击：选中某个条目（usize::MAX = 点击空白 = 取消选中）,
    Click(usize),
    /// 双击：下钻进入子目录
    DoubleClick(usize),
}
