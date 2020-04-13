from typing import *
import json
import math
import sys
import traceback
import tkinter
import tkinter.filedialog
import os
import time
import traceback
import gc

import glfw
from imgui.integrations.glfw import GlfwRenderer
from OpenGL import GL as gl

import wafel.imgui as ig
from wafel.core import *
from wafel.model import Model
from wafel.frame_sheet import FrameSheet
from wafel.variable import Variable
from wafel.variable_explorer import VariableExplorer
from wafel.variable_format import Formatters, EnumFormatter, DataFormatters
from wafel.format_m64 import load_m64, save_m64
from wafel.tas_metadata import TasMetadata
from wafel.window import open_window_and_run
import wafel.ui as ui
from wafel.local_state import use_state, use_state_with
from wafel.util import *
import wafel.config as config
from wafel.bindings import *
from wafel.loading import *
from wafel.edit import Edits


DEFAULT_FRAME_SHEET_VARS = [
  'input-button-a',
  'input-button-b',
  'input-button-z',
  'mario-action',
  'mario-vel-f',
]

if config.dev_mode:
  DEFAULT_FRAME_SHEET_VARS.insert(0, 'wafel-script')


class SequenceFile:
  FILE_TYPES = [
    # ('Wafel TAS', '*.wafi'),
    ('Mupen64 TAS', '*.m64'),
    ('All files', '*'),
  ]

  @staticmethod
  def from_filename(filename: str) -> 'SequenceFile':
    _, ext = os.path.splitext(filename)
    if ext == '.wafi':
      return SequenceFile(filename, 'wafi')
    elif ext == '.m64':
      return SequenceFile(filename, 'm64')
    else:
      raise NotImplementedError(ext) # TODO: User message

  def __init__(self, filename: str, type_: str) -> None:
    self.filename = filename
    self.type = type_


class View:

  def __init__(self, model: Model) -> None:
    self.loading: Optional[Loading[None]] = None
    self.model = model
    self.epoch = 0
    self.tkinter_root = tkinter.Tk()
    self.tkinter_root.withdraw()

    self.dbg_frame_advance = False

    self.file: Optional[SequenceFile] = None
    self.reload()


  def reload(self) -> None:
    if self.loading is not None:
      return
    self.loading = self._reload()


  def _reload(self) -> Loading[None]:
    if self.file is None:
      metadata = TasMetadata('us', 'Untitled TAS', 'Unknown author(s)', 'Made using Wafel')
      edits = Edits()
    elif self.file.type == 'm64':
      metadata, edits = load_m64(self.file.filename)
    else:
      raise NotImplementedError(self.file.type)
    self.metadata = metadata
    yield from self.model.load(metadata.game_version, edits)

    self.reload_ui()


  def change_version(self, version: str) -> None:
    if self.loading is not None:
      return
    self.loading = self._change_version(version)


  def _change_version(self, version: str) -> Loading[None]:
    self.metadata.game_version = version
    yield from self.model.change_version(version)
    self.reload_ui()


  def reload_ui(self) -> None:
    self.show_debug_pane = config.dev_mode
    self.formatters = DataFormatters(self.model.data_variables)
    self.formatters[Variable('mario-action')] = EnumFormatter(self.model.action_names)

    self.frame_sheets: List[FrameSheet] = [
      FrameSheet(self.model, self.model.accessor, self.model.range_edit_accessor, self.model, self.formatters),
    ]
    for var_name in DEFAULT_FRAME_SHEET_VARS:
      self.frame_sheets[0].append_variable(Variable(var_name))

    self.variable_explorer = VariableExplorer(self.model, self.formatters)

    self.epoch += 1


  def save(self) -> None:
    assert self.file is not None
    if self.file.type == 'm64':
      save_m64(self.file.filename, self.metadata, self.model.edits)
    else:
      raise NotImplementedError(self.file.type)


  def render_left_column(self, framebuffer_size: Tuple[int, int]) -> None:
    total_height = ig.get_window_height() - ig.get_frame_height() # subtract menu bar
    slider_space = 45

    wall_hitbox_radius = use_state('wall-hitbox-radius', 50)
    wall_hitbox_options = [0, 24, 50, 110]

    hovered_surface: Ref[Optional[int]] = use_state('hovered-surface', None)
    new_hovered_surface: Optional[int] = None
    hidden_surfaces_by_area = \
      use_state('hidden-surfaces', cast(Dict[Tuple[int, int], Set[int]], {})).value

    current_area = (
      dcast(int, self.model.get(Variable('level-num').at(frame=self.model.selected_frame))),
      dcast(int, self.model.get(Variable('area-index').at(frame=self.model.selected_frame))),
    )
    hidden_surfaces = hidden_surfaces_by_area.setdefault(current_area, set())

    in_game_view = use_state('in-game-view', False)

    log.timer.begin('gview1')
    ig.begin_child(
      'Game View 1',
      height=int(total_height // 2) - slider_space // 2,
      border=True,
    )
    if config.dev_mode and in_game_view.value:
      ui.render_game_view_in_game('game-view-1', framebuffer_size, self.model)
      hovered_surface_1 = None
    else:
      hovered_surface_1 = ui.render_game_view_rotate(
        'game-view-1',
        framebuffer_size,
        self.model,
        wall_hitbox_radius.value,
        hovered_surface.value,
        hidden_surfaces,
      )

    ig.set_cursor_pos((10.0, ig.get_window_height() - 30))
    ig.text('wall radius')
    ig.same_line()
    ig.push_item_width(50)
    _, index = ig.combo(
      '##wall-hitbox-radius',
      wall_hitbox_options.index(wall_hitbox_radius.value),
      list(map(str, wall_hitbox_options)),
    )
    wall_hitbox_radius.value = wall_hitbox_options[index]
    ig.pop_item_width()

    if config.dev_mode:
      ig.same_line()
      if ig.button('Secret hype'):
        in_game_view.value = not in_game_view.value

    ig.end_child()
    log.timer.end()

    log.timer.begin('gview2')
    ig.begin_child(
      'Game View 2',
      height=int(total_height // 2) - slider_space // 2,
      border=True,
    )
    hovered_surface_2 = ui.render_game_view_birds_eye(
      'game-view-2',
      framebuffer_size,
      self.model,
      wall_hitbox_radius.value,
      hovered_surface.value,
      hidden_surfaces,
    )
    ig.end_child()
    log.timer.end()

    new_hovered_surface = hovered_surface_1 or hovered_surface_2
    if new_hovered_surface is not None and ig.is_mouse_clicked(1):
      ig.open_popup('surface-ctx')
      hovered_surface.value = new_hovered_surface

    if ig.begin_popup('surface-ctx'):
      if hovered_surface.value is not None:
        if hovered_surface.value in hidden_surfaces:
          if ig.menu_item('Show')[0]:
            hidden_surfaces.remove(hovered_surface.value)
        else:
          if ig.menu_item('Hide')[0]:
            hidden_surfaces.add(hovered_surface.value)
        if ig.menu_item('Properties')[0]:
          self.variable_explorer.open_surface_tab(hovered_surface.value)
      ig.end_popup()
    else:
      hovered_surface.value = new_hovered_surface

    if hovered_surface.value is not None and ig.is_mouse_clicked(2):
      if hovered_surface.value in hidden_surfaces:
        hidden_surfaces.remove(hovered_surface.value)
      else:
        hidden_surfaces.add(hovered_surface.value)


    speed_options = [0.05, 0.25, 0.5, 1, 2, 4]
    saved_play_direction = use_state('saved-play-direction', 0)
    saved_speed_index = use_state('saved-speed-index', 3)

    play_direction = saved_play_direction.value
    speed_index = saved_speed_index.value

    if play_direction == 0:
      frame_advance = 0
      play_override = 0

      def control(name: str, speed: int) -> None:
        nonlocal frame_advance, play_override
        x = input_down_gradual(name, 0.25)
        if x == 1.0:
          play_override = speed
        elif input_pressed(name):
          frame_advance += speed
      control('frame-next', 1)
      control('frame-next-alt', 1)
      control('frame-prev', -1)
      control('frame-prev-alt', -1)
      control('frame-next-fast', 10)
      control('frame-prev-fast', -10)

      if play_override != 0:
        if abs(play_override) in speed_options:
          speed_index = speed_options.index(abs(play_override))
        else:
          speed_index = len(speed_options) - 1
        play_direction = 1 if play_override > 0 else -1
      else:
        self.model.selected_frame += frame_advance

    else:
      if input_down('frame-next') or input_down('frame-next-alt'):
        if play_direction == 1:
          speed_index += 1
        else:
          play_direction = -play_direction
      elif input_down('frame-prev') or input_down('frame-prev-alt'):
        if play_direction == -1:
          speed_index += 1
        else:
          play_direction = -play_direction
      elif input_down('frame-next-fast'):
        if play_direction == 1:
          speed_index += 2
        else:
          play_direction = -play_direction
          speed_index += 1
      elif input_down('frame-prev-fast'):
        if play_direction == -1:
          speed_index += 2
        else:
          play_direction = -play_direction
          speed_index += 1
      speed_index = min(max(speed_index, 0), len(speed_options) - 1)

    self.model.play_speed = play_direction * speed_options[speed_index]
    self.model.playback_mode = saved_play_direction.value != 0

    def play_button(label: str, direction: int) -> None:
      disabled = play_direction == direction
      if ig.disableable_button(label, enabled=play_direction != direction):
        saved_play_direction.value = direction

    play_button('<|', -1)
    ig.same_line()
    play_button('||', 0)
    ig.same_line()
    play_button('|>', 1)
    ig.same_line()

    ig.push_item_width(63)
    changed, new_index = ig.combo(
      '##speed-option',
      speed_index,
      [str(s) + 'x' for s in speed_options],
    )
    ig.pop_item_width()
    if changed:
      saved_speed_index.value = new_index

    if input_pressed('playback-play'):
      if saved_play_direction.value == 0:
        saved_play_direction.value = 1
      else:
        saved_play_direction.value = 0
    if input_pressed('playback-rewind'):
      if saved_play_direction.value == 0:
        saved_play_direction.value = -1
      else:
        saved_play_direction.value = 0
    if input_pressed('playback-speed-up'):
      saved_speed_index.value = min(saved_speed_index.value + 1, len(speed_options) - 1)
    if input_pressed('playback-slow-down'):
      saved_speed_index.value = max(saved_speed_index.value - 1, 0)


    ig.same_line()
    new_frame = ui.render_frame_slider(
      'frame-slider',
      self.model.selected_frame,
      len(self.model.edits),
      self.model.timeline.slot_manager.get_loaded_frames() if self.show_debug_pane else [],
    )
    if new_frame is not None:
      self.model.selected_frame = new_frame.value


  def render_right_column(self) -> None:
    total_height = ig.get_window_height()

    if self.show_debug_pane:
      ig.push_id('debug-pane')
      ig.begin_child('##pane', height=int(ig.get_window_height() * 0.15))
      ig.columns(2)
      ig.set_column_width(-1, ig.get_window_width() - 300)

      ig.begin_child('##log')
      def init_log() -> List[str]:
        messages = []
        log.subscribe(lambda msg: messages.append(str(msg)))
        return messages
      messages = use_state_with('messages', init_log).value
      prev_length = use_state('prev-length', 0)
      total_height -= ig.get_window_height()
      if prev_length.value != len(messages):
        prev_length.value = len(messages)
        ig.set_scroll_y(ig.get_scroll_max_y() + ig.get_window_height())
      for message in messages:
        ig.text(message)
      ig.end_child()

      ig.next_column()

      for line in log.timer.format(log.timer.get_summaries()):
        ig.text(line)

      ig.columns(1)
      ig.end_child()
      ig.pop_id()

    log.timer.begin('fsheet')
    frame_sheet = self.frame_sheets[0]
    ig.set_next_window_content_size(frame_sheet.get_content_width(), 0)
    ig.begin_child(
      'Frame Sheet##' + str(self.epoch) + '-0',
      height=int(total_height * 0.7),
      flags=ig.WINDOW_HORIZONTAL_SCROLLING_BAR,
    )
    frame_sheet.render()
    ig.end_child()

    if ig.begin_drag_drop_target():
      payload = ig.accept_drag_drop_payload('ve-var')
      if payload is not None:
        frame_sheet.append_variable(Variable.from_bytes(payload))
      ig.end_drag_drop_target()
    log.timer.end()

    log.timer.begin('varexp')
    ig.begin_child('Variable Explorer', border=True)
    self.variable_explorer.render('variable-explorer')
    ig.end_child()
    log.timer.end()


  def tkinter_lift(self) -> None:
    self.tkinter_root.attributes('-topmost', True)
    self.tkinter_root.lift()


  def ask_save_filename(self) -> bool:
    self.tkinter_lift()
    filename = tkinter.filedialog.asksaveasfilename(
      defaultext='.m64',
      filetypes=SequenceFile.FILE_TYPES,
    ) or None
    if filename is None:
      return False
    self.file = SequenceFile.from_filename(filename)
    return True


  def render_menu_bar(self) -> None:
    open_popup = None

    if ig.begin_menu_bar():
      if ig.begin_menu('File'):
        if ig.menu_item('New')[0]:
          self.file = None
          self.reload()

        if ig.menu_item('Open')[0]:
          self.tkinter_lift()
          filename = tkinter.filedialog.askopenfilename() or None
          if filename is not None:
            self.file = SequenceFile.from_filename(filename)
            self.reload()

        if ig.menu_item('Save')[0]:
          if self.file is None:
            if self.ask_save_filename():
              self.save()
          else:
            self.save()

        if ig.menu_item('Save as')[0]:
          if self.ask_save_filename():
            self.save()

        if ig.begin_menu('Game version'):
          versions = [
            ('US', 'us'),
            ('J', 'jp'),
          ]
          for label, version in versions:
            if ig.menu_item(label, selected = self.metadata.game_version == version)[0]:
              self.change_version(version)
          ig.end_menu()

        ig.end_menu()

      if ig.begin_menu('Settings'):
        if ig.menu_item('Controller')[0]:
          open_popup = 'Controller##settings-controller'
        if ig.menu_item('Key bindings')[0]:
          open_popup = 'Key bindings##settings-key-bindings'
        ig.end_menu()

      ig.end_menu_bar()

    if open_popup is not None:
      ig.open_popup(open_popup)


  def compute_stick_from_controller(self, cx: float, cy: float) -> Tuple[int, int]:
    if abs(cx) < 8 / 128:
      cx = 0
    if abs(cy) < 8 / 128:
      cy = 0
    camera_angle = dcast(int, self.model.get(Variable('camera-yaw').at(frame=self.model.selected_frame)) or 0) + 0x8000
    up_angle = self.model.input_up_yaw
    if up_angle is None:
      up_angle = camera_angle
    rotation = (up_angle - camera_angle) / 0x8000 * math.pi
    sx = math.cos(rotation) * cx - math.sin(rotation) * cy
    sy = math.sin(rotation) * cx + math.cos(rotation) * cy
    stick_x = min(max(int(sx * 128), -128), 127)
    stick_y = min(max(int(sy * 128), -128), 127)
    return stick_x, stick_y


  def handle_controller(self) -> None:
    ig.push_id('controller-inputs')

    buttons_enabled = use_state('buttons-enabled', False)
    stick_enabled = use_state('stick-enabled', False)

    def add_callbacks() -> Ref[bool]:
      input_edit = Ref(False)
      def disable_controller(*args, **kwargs) -> None:
        if not input_edit.value:
          buttons_enabled.value = False
          stick_enabled.value = False
      self.model.timeline.on_invalidation(disable_controller)
      def frame_change(*args, **kwargs) -> None:
        if self.model.play_speed == 0.0:
          disable_controller()
      self.model.on_selected_frame_change(frame_change)
      return input_edit
    input_edit = use_state_with('initialize', add_callbacks).value

    prev_play_speed = use_state('prev-play-speed', 0.0)
    if self.model.play_speed != prev_play_speed.value:
      buttons_enabled.value = False
      stick_enabled.value = False
    prev_play_speed.value = self.model.play_speed

    controller_button_values = {
      'input-button-a': input_down('n64-A'),
      'input-button-b': input_down('n64-B'),
      'input-button-z': input_down('n64-Z'),
      'input-button-s': input_down('n64-S'),
      'input-button-l': input_down('n64-L'),
      'input-button-r': input_down('n64-R'),
      'input-button-cu': input_down('n64-C^'),
      'input-button-cl': input_down('n64-C<'),
      'input-button-cr': input_down('n64-C>'),
      'input-button-cd': input_down('n64-Cv'),
      'input-button-du': input_down('n64-D^'),
      'input-button-dl': input_down('n64-D<'),
      'input-button-dr': input_down('n64-D>'),
      'input-button-dd': input_down('n64-Dv'),
    }
    if any(controller_button_values.values()):
      buttons_enabled.value = True
      stick_enabled.value = True
    for variable_name, new_button_value in controller_button_values.items():
      variable = Variable(variable_name).at(frame=self.model.selected_frame)
      button_value = self.model.get(variable)
      if buttons_enabled.value and button_value != new_button_value:
        input_edit.value = True
        self.model.edits.edit(variable, new_button_value)
        input_edit.value = False

    controller_stick_values = (
      input_float('n64->') - input_float('n64-<'),
      input_float('n64-^') - input_float('n64-v'),
    )
    if any(controller_stick_values):
      stick_enabled.value = True
      buttons_enabled.value = True
    if stick_enabled.value:
      stick_x_var = Variable('input-stick-x').at(frame=self.model.selected_frame)
      stick_y_var = Variable('input-stick-y').at(frame=self.model.selected_frame)
      new_stick = self.compute_stick_from_controller(*controller_stick_values)
      stick = (self.model.get(stick_x_var), self.model.get(stick_y_var))
      if stick != new_stick:
        input_edit.value = True
        self.model.edits.edit(stick_x_var, new_stick[0])
        self.model.edits.edit(stick_y_var, new_stick[1])
        input_edit.value = False

    ig.pop_id()

  def render(self) -> None:
    ig.push_id(str(self.epoch))

    if self.loading is not None:
      try:
        progress = None
        start_time = time.time()
        while time.time() - start_time < 1/60:
          progress = next(self.loading)
          if progress.progress == 0.0:
            break
      except StopIteration:
        self.loading = None
      if progress is not None:
        window_size = ig.get_window_size()
        width = 500
        ig.set_cursor_pos((window_size.x / 2 - width / 2, window_size.y / 2 - 60))
        ui.render_loading_bar('loading-bar', progress, width)
      ig.pop_id()
      return

    if ig.is_key_pressed(ord('`')):
      self.show_debug_pane = not self.show_debug_pane

    self.handle_controller()


    prev_frame_time = use_state_with('prev-frame-time', time.time)
    accum_time = use_state('accum-time', 0.0)
    now = time.time()
    accum_time.value += now - prev_frame_time.value
    prev_frame_time.value = now

    play_speed = self.model.play_speed
    if play_speed == 0.0:
      accum_time.value = 0
    else:
      target_fps = 30 * abs(play_speed)
      target_dt = 1 / target_fps
      updates = 0
      while accum_time.value >= target_dt and updates < 20:
        accum_time.value -= target_dt
        self.model.selected_frame += 1 if play_speed > 0 else -1
        self.handle_controller()
        updates += 1


    ig_window_size = ig.get_window_size()
    window_size = (int(ig_window_size.x), int(ig_window_size.y))
    self.render_menu_bar()

    if ig.begin_popup_modal('Controller##settings-controller', True, ig.WINDOW_NO_RESIZE)[0]:
      render_controller_settings('content')
      ig.end_popup_modal()
    if ig.begin_popup_modal('Key bindings##settings-key-bindings', True, ig.WINDOW_NO_RESIZE)[0]:
      render_key_binding_settings('content')
      ig.end_popup_modal()

    ig.columns(2)
    self.render_left_column(window_size)
    ig.next_column()
    self.render_right_column()
    ig.columns(1)

    ig.pop_id()


def run() -> None:
  model = Model()
  view = None
  error = None

  def do_render(id: str) -> None:
    nonlocal view

    if view is None:
      view = View(model)
      if config.dev_mode:
        view.loading = None
        view.file = SequenceFile('test_files/22stars.m64', 'm64')
        view.reload()
    ig.push_id(id)

    log.timer.begin('render')
    view.render()
    log.timer.end()

    ig.pop_id()


    last_fps_time = use_state_with('last-fps-time', lambda: time.time())
    frame_count = use_state('frame-count', 0)
    fps = use_state('fps', 0.0)

    if hasattr(model, 'timeline'):
      frame_count.value += 1
      if time.time() > last_fps_time.value + 5:
        fps.value = frame_count.value / (time.time() - last_fps_time.value)
        last_fps_time.value = time.time()
        frame_count.value = 0
        log.info(
          f'mspf: {int(1000 / fps.value * 10) / 10} ({int(fps.value)} fps) - ' +
          f'cache={model.timeline.data_cache.get_size() // 1024}KB'
        )

      log.timer.begin('balance')
      model.timeline.balance_distribution(1/120)
      log.timer.end()

  # TODO: Clean up (use local_state)
  def render(id: str) -> None:
    nonlocal error

    if error is not None:
      message = error.strip()
      ig.text('Wafel has crashed. Cause:')
      lines = message.split('\n')
      ig.input_text_multiline(
        '##error-msg',
        message,
        len(message) + 1,
        max(map(len, lines)) * 10,
        (len(lines) + 1) * ig.get_text_line_height() + 6,
      )
      ig.text('The horribleness of Mupen may also somehow factor into this.')

      ig.dummy(10, 10)

      if ig.button('Exit'):
        log.info('Aborted')
        sys.exit(1)
      ig.same_line()
      if ig.button('Try to save'):
        if view.ask_save_filename():
          view.save()
      # ig.same_line()
      # if view is not None and ig.button('Try to continue (mad lads only)'):
      #   view.reload_ui()
      #   error = None
      return

    try:
      log.timer.begin_frame()
      ig.try_render(lambda: do_render(id))
    except:
      error = traceback.format_exc()
      log.error('Caught: ' + error)
    finally:
      log.timer.end_frame()

      # # TODO: Should enable in non-dev mode, but should throttle it
      # if config.dev_mode:
      #   summary = log.timer.get_frame_summary()
      #   if summary[('frame',)].time >= 100:
      #     log.warn('Spike:\n' + '\n'.join(log.timer.format(summary)))

  open_window_and_run(render, maximize=True)
