import imgui as ig

from wafel.model import Model


class FrameSlider:
  def __init__(self, model: Model) -> None:
    self.model = model

  def render(self) -> None:
    pos = ig.get_cursor_pos()
    width = ig.get_column_width(-1) - 2 * ig.get_style().item_spacing[0]

    ig.push_item_width(width)
    changed, frame = ig.slider_int(
      '##frame-slider',
      self.model.selected_frame,
      0,
      len(self.model.timeline) - 1,
    )
    ig.pop_item_width()

    if changed:
      self.model.selected_frame = frame

    dl = ig.get_window_draw_list()
    for frame in self.model.timeline.get_loaded_frames():
      line_pos = pos[0] + frame / len(self.model.timeline) * width
      dl.add_line(
        line_pos, pos[1],
        line_pos, pos[1] + 18,
        ig.get_color_u32_rgba(1, 0, 0, 1),
      )
