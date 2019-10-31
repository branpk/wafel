from typing import *

import imgui as ig

from wafel.model import Model
from wafel.core import ObjectId, Variable, VariableGroup, VariableParam, VariableEdit
from wafel.variable_display import VariableDisplayAction, display_variable_data
from wafel.variable_format import Formatters
from wafel.frame_sheet import CellEditState


class TabId:
  def __init__(self, name: str, object_id: Optional[ObjectId] = None) -> None:
    self.name = name
    self.object_id = object_id

  def __eq__(self, other: object) -> bool:
    if not isinstance(other, TabId):
      return False
    return self.name == other.name and self.object_id == other.object_id

  def __hash__(self) -> int:
    return hash((self.name, self.object_id))


class VariableCell:
  def __init__(self, tab: TabId, variable: Variable, frame: int) -> None:
    self.tab = tab
    self.variable = variable
    self.frame = frame

  def __eq__(self, other: object) -> bool:
    if not isinstance(other, VariableCell):
      return False
    return self.tab == other.tab and \
      self.variable == other.variable and \
      self.frame == other.frame

  def __hash__(self) -> int:
    return hash((self.tab, self.variable, self.frame))


class JoystickControl:
  def __init__(self):
    self.start_value: Optional[Tuple[float, float]] = None

  def update(self, value: Tuple[float, float], drag: Tuple[float, float]) -> Tuple[float, float]:
    if self.start_value is None:
      self.start_value = value
    return (self.start_value[0] + drag[0], self.start_value[1] + drag[1])

  def reset(self):
    self.start_value = None


class VariableExplorer:

  def __init__(self, model: Model, formatters: Formatters) -> None:
    self.model = model
    self.formatters = formatters
    self.open_tabs: List[TabId] = []
    self.rendered = False
    self.cell_edit_state: CellEditState[VariableCell] = CellEditState()

    fixed_tabs = [
      TabId('Input'),
      TabId('Mario'),
      TabId('Misc'),
      TabId('Objects'),
    ]
    for tab in fixed_tabs:
      self.open_tab(tab)

    self.current_tab = self.open_tabs[0]
    self.joystick_control = JoystickControl()


  def open_tab(self, tab: TabId) -> None:
    if tab not in self.open_tabs:
      self.open_tabs.append(tab)
    self.current_tab = tab


  def close_tab(self, tab: TabId) -> None:
    if self.current_tab == tab:
      # TODO
      pass
    if tab in self.open_tabs:
      self.open_tabs.remove(tab)


  def get_tab_label(self, tab: TabId) -> str:
    if tab.object_id is not None:
      state = self.model.timeline[self.model.selected_frame]
      object_type = self.model.get_object_type(state, tab.object_id)
      if object_type is None:
        return str(tab.object_id)
      else:
        return str(tab.object_id) + ': ' + object_type.name

    return tab.name


  def render_objects_tab(self) -> None:
    button_size = 50
    window_left = ig.get_window_position()[0]
    window_right = window_left + ig.get_window_content_region_max()[0]
    prev_item_right = window_left
    style = ig.get_style()

    for slot in range(240):
      item_right = prev_item_right + style.item_spacing[0] + button_size
      if item_right > window_right:
        prev_item_right = window_left
      elif slot != 0:
        ig.same_line()
      prev_item_right = prev_item_right + style.item_spacing[0] + button_size

      object_id = slot
      object_type = self.model.get_object_type(
        self.model.timeline[self.model.selected_frame],
        object_id,
      )
      if object_type is None:
        label = str(slot)
      else:
        label = str(slot) + '\n' + object_type.name

      if ig.button(label + '##slot-' + str(slot), 50, 50):
        self.open_tab(TabId('_object', object_id))


  def get_variables_for_tab(self, tab: TabId) -> List[Variable]:
    if tab.object_id is None:
      return self.model.variables.group(VariableGroup(tab.name))

    state = self.model.timeline[self.model.selected_frame]
    object_type = self.model.get_object_type(state, tab.object_id)
    if object_type is None:
      return []

    return [
      var.at_object(tab.object_id)
        for var in self.model.variables.group(VariableGroup.object(object_type.name))
    ]


  def render_variable(self, tab: TabId, variable: Variable) -> None:
    frame = self.model.selected_frame
    state = self.model.timeline[frame]
    cell = VariableCell(tab, variable, state.frame)
    data = variable.get({ VariableParam.STATE: state })
    del state

    ig.selectable(variable.label + '##ve-label-' + str(hash(cell)), width=80)

    if ig.begin_drag_drop_source():
      ig.text(variable.label)
      ig.set_drag_drop_payload('ve-var', variable.id.to_bytes())
      ig.end_drag_drop_source()

    ig.same_line()

    cell_width = 80
    cell_height = ig.get_text_line_height() + 2 * ig.get_style().frame_padding[1]

    cell_cursor_pos = ig.get_cursor_pos()
    cell_cursor_pos = (
      cell_cursor_pos[0] + ig.get_window_position()[0],
      cell_cursor_pos[1] + ig.get_window_position()[1] - ig.get_scroll_y(),
    )

    def on_edit(data: Any) -> None:
      self.model.edits.add(frame, VariableEdit(variable, data))

    action = display_variable_data(
      've-var-' + str(hash(cell)),
      data,
      self.formatters[variable],
      (cell_width, cell_height),
      self.cell_edit_state.get(cell),
      on_edit = on_edit,
    )

    if action == VariableDisplayAction.BEGIN_EDIT:
      self.cell_edit_state.begin_edit(cell)
    elif action == VariableDisplayAction.END_EDIT:
      self.cell_edit_state.end_edit()

    if ig.is_item_hovered() and ig.is_mouse_down(2):
      self.model.edits.reset(frame, variable)

    if self.model.edits.is_edited(frame, variable):
      dl = ig.get_window_draw_list()
      spacing = ig.get_style().item_spacing
      spacing = (spacing[0] / 2, spacing[1] / 2)
      dl.add_rect(
        cell_cursor_pos[0] - spacing[0],
        cell_cursor_pos[1] - spacing[1],
        cell_cursor_pos[0] + cell_width + spacing[0] - 1,
        cell_cursor_pos[1] + cell_height + spacing[1] - 1,
        ig.get_color_u32_rgba(0.8, 0.6, 0, 1),
      )


  def render_stick_control(self, stick_x: Variable, stick_y: Variable) -> None:
    dl = ig.get_window_draw_list()

    padding = 20
    size = min(
      ig.get_column_width() - ig.get_style().scrollbar_size - 2 * padding,
      ig.get_window_height() - 2 * padding,
      200,
    )
    top_left = ig.get_cursor_pos()
    top_left = (
      top_left[0] + ig.get_window_position()[0] + padding,
      top_left[1] + ig.get_window_position()[1] - ig.get_scroll_y() + padding,
    )

    dl.add_rect_filled(
      top_left[0],
      top_left[1],
      top_left[0] + size,
      top_left[1] + size,
      ig.get_color_u32_rgba(0, 0, 0, 0.3),
    )

    offset = (
      (self.model.get(stick_x) + 128) / 255 * size,
      (1 - (self.model.get(stick_y) + 128) / 255) * size,
    )
    dl.add_line(
      top_left[0] + size / 2,
      top_left[1] + size / 2,
      top_left[0] + offset[0],
      top_left[1] + offset[1],
      ig.get_color_u32_rgba(1, 1, 1, 0.5),
    )

    button_size = 20
    button_pos = ig.get_cursor_pos()
    button_pos = (
      padding + button_pos[0] + offset[0] - button_size / 2,
      padding + button_pos[1] + offset[1] - button_size / 2,
    )
    ig.set_cursor_pos(button_pos)
    ig.button('##ve-stick-control-button', button_size, button_size)

    if ig.is_item_active():
      new_offset = self.joystick_control.update(offset, ig.get_mouse_drag_delta(lock_threshold=0))

      new_stick_x = new_offset[0] / size * 255 - 128
      new_stick_y = (1 - new_offset[1] / size) * 255 - 128
      new_stick_x = min(max(int(new_stick_x), -128), 127)
      new_stick_y = min(max(int(new_stick_y), -128), 127)

      if new_stick_x != self.model.get(stick_x) or new_stick_y != self.model.get(stick_y):
        self.model.edits.add(self.model.selected_frame, VariableEdit(stick_x, new_stick_x))
        self.model.edits.add(self.model.selected_frame, VariableEdit(stick_y, new_stick_y))

    else:
      self.joystick_control.reset()


  def render_input_tab(self, tab: TabId) -> None:
    ig.columns(2)
    ig.set_column_width(-1, 160)

    variables = self.get_variables_for_tab(tab)
    for variable in variables:
      self.render_variable(tab, variable)

    ig.next_column()

    self.render_stick_control(
      self.model.variables['input-stick-x'],
      self.model.variables['input-stick-y'],
    )

    ig.columns(1)


  def render_variable_tab(self, tab: TabId) -> None:
    variables = self.get_variables_for_tab(tab)
    for variable in variables:
      self.render_variable(tab, variable)


  def render_tab_contents(self, tab: TabId) -> None:
    if tab.name != 'Input':
      self.joystick_control.reset()

    if tab.name == 'Objects':
      self.render_objects_tab()
    elif tab.name == 'Input':
      self.render_input_tab(tab)
    else:
      self.render_variable_tab(tab)


  def render(self) -> None:
    ig.columns(2)
    if not self.rendered:
      self.rendered = True
      ig.set_column_width(-1, 120)

    ig.begin_child('Variable Explorer Tabs')
    for tab in self.open_tabs:
      _, selected = ig.selectable(
        self.get_tab_label(tab) + '##' + str(id(tab)),
        self.current_tab == tab,
      )
      if selected:
        self.current_tab = tab
    ig.end_child()

    ig.next_column()

    ig.begin_child('Variable Explorer Content')
    self.render_tab_contents(self.current_tab)
    ig.end_child()

    ig.columns(1)
