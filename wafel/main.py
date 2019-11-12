from typing import *
from ctypes import *
import json
import math
import sys
import traceback
import tkinter
import tkinter.filedialog
import os

import glfw
import imgui as ig
from imgui.integrations.glfw import GlfwRenderer
from OpenGL import GL as gl

from wafel.graphics import *
from wafel.core import *
from wafel.model import Model
from wafel.frame_sheet import FrameSheet
from wafel.variable_explorer import VariableExplorer
from wafel.game_view import GameView
from wafel.frame_slider import *
from wafel.variable_format import Formatters
from wafel.format_m64 import load_m64, save_m64
from wafel.format_wafi import load_wafi, save_wafi
from wafel.tas_metadata import TasMetadata


DEFAULT_FRAME_SHEET_VARS = [
  'input-stick-x',
  'input-stick-y',
  'input-button-a',
  'input-button-b',
  'input-button-z',
]


class SequenceFile:
  FILE_TYPES = [
    ('Wafel TAS', '*.wafi'),
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
    self.model = model
    self.epoch = 0
    self.tkinter_root = tkinter.Tk()
    self.tkinter_root.withdraw()

    self.dbg_frame_advance = False

    self.file: Optional[SequenceFile] = None
    self.reload()


  def reload(self) -> None:
    if self.file is None:
      metadata = TasMetadata('us', 'Untitled TAS', 'Unknown author(s)', 'Made using Wafel')
      edits = Edits()
    elif self.file.type == 'wafi':
      metadata, edits = load_wafi(self.file.filename)
    elif self.file.type == 'm64':
      metadata, edits = load_m64(self.file.filename)
    else:
      raise NotImplementedError(self.file.type)
    self.metadata = metadata
    self.model.load(metadata.game_version, edits)

    self.reload_ui()


  def reload_ui(self) -> None:
    self.formatters = Formatters()

    self.frame_sheets: List[FrameSheet] = [FrameSheet(self.model, self.formatters)]
    for var_name in DEFAULT_FRAME_SHEET_VARS:
      self.frame_sheets[0].append_variable(self.model.variables[var_name])

    self.variable_explorer = VariableExplorer(self.model, self.formatters)
    self.game_views: List[GameView] = [
      GameView(self.model, CameraMode.ROTATE),
      GameView(self.model, CameraMode.BIRDS_EYE),
    ]
    self.frame_slider = FrameSlider(self.model)

    self.epoch += 1


  def save(self) -> None:
    assert self.file is not None
    if self.file.type == 'wafi':
      save_wafi(self.file.filename, self.metadata, self.model.edits)
    elif self.file.type == 'm64':
      save_m64(self.file.filename, self.metadata, self.model.edits)
    else:
      raise NotImplementedError(self.file.type)


  def render_left_column(self, framebuffer_size: Tuple[int, int]) -> None:
    total_height = ig.get_window_height() - ig.get_frame_height() # subtract menu bar
    slider_space = 45

    ig.begin_child(
      'Game View 1',
      height=int(total_height // 2) - slider_space // 2,
      border=True,
    )
    self.game_views[0].render(framebuffer_size)
    ig.end_child()

    ig.begin_child(
      'Game View 2',
      height=int(total_height // 2) - slider_space // 2,
      border=True,
    )
    self.game_views[1].render(framebuffer_size)
    ig.end_child()

    self.frame_slider.render()


  def render_right_column(self) -> None:
    frame_sheet = self.frame_sheets[0]
    ig.set_next_window_content_size(frame_sheet.get_content_width(), 0)
    ig.begin_child(
      'Frame Sheet##' + str(self.epoch) + '-0',
      height=int(ig.get_window_height() * 0.7),
      flags=ig.WINDOW_HORIZONTAL_SCROLLING_BAR,
    )
    frame_sheet.render()
    ig.end_child()

    if ig.begin_drag_drop_target():
      payload = ig.accept_drag_drop_payload('ve-var')
      if payload is not None:
        variable = self.model.variables[VariableId.from_bytes(payload)]
        frame_sheet.append_variable(variable)
      ig.end_drag_drop_target()

    ig.begin_child('Variable Explorer', border=True)
    self.variable_explorer.render()
    ig.end_child()


  def ask_save_filename(self) -> bool:
    filename = tkinter.filedialog.asksaveasfilename(
      defaultext='.wafi',
      filetypes=SequenceFile.FILE_TYPES,
    ) or None
    if filename is None:
      return False
    self.file = SequenceFile.from_filename(filename)
    return True


  def render_menu_bar(self) -> None:
    if ig.begin_menu_bar():
      if ig.begin_menu('File'):
        if ig.menu_item('New')[0]:
          self.file = None
          self.reload()

        if ig.menu_item('Open')[0]:
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
              self.metadata.game_version = version
              self.model.change_version(version)
              self.reload_ui()
          ig.end_menu()

        ig.end_menu()
      ig.end_menu_bar()


  def render(self, window_size: Tuple[int, int]) -> None:
    ig.set_next_window_position(0, 0)
    ig.set_next_window_size(*window_size)
    ig.begin(
      'Main##' + str(self.epoch),
      False,
      ig.WINDOW_NO_SAVED_SETTINGS | ig.WINDOW_NO_RESIZE | ig.WINDOW_NO_TITLE_BAR | ig.WINDOW_MENU_BAR,
    )

    self.render_menu_bar()

    ig.columns(2)
    self.render_left_column(window_size)
    ig.next_column()
    self.render_right_column()
    ig.columns(1)

    ig.end()


  def dbg_is_key_pressed(self, key: int) -> bool:
    if not hasattr(self, 'dbg_keys_down'):
      self.dbg_keys_down = set()

    if ig.is_key_down(key):
      pressed = key not in self.dbg_keys_down
      self.dbg_keys_down.add(key)
      return pressed
    else:
      if key in self.dbg_keys_down:
        self.dbg_keys_down.remove(key)
      return False


def render(window, ig_renderer, view: View) -> None:
  # TODO: Move keyboard handling somewhere else
  # TODO: Make this work when holding down mouse button
  model = view.model
  ig.get_io().key_repeat_rate = 1/30
  if not ig.get_io().want_capture_keyboard:
    if ig.is_key_pressed(ig.get_key_index(ig.KEY_DOWN_ARROW)) or \
        ig.is_key_pressed(ig.get_key_index(ig.KEY_RIGHT_ARROW)):
      model.selected_frame += 1
    if ig.is_key_pressed(ig.get_key_index(ig.KEY_UP_ARROW)) or \
        ig.is_key_pressed(ig.get_key_index(ig.KEY_LEFT_ARROW)):
      model.selected_frame -= 1
    if ig.is_key_pressed(ig.get_key_index(ig.KEY_PAGE_DOWN)):
      model.selected_frame += 5
    if ig.is_key_pressed(ig.get_key_index(ig.KEY_PAGE_UP)):
      model.selected_frame -= 5

    if view.dbg_is_key_pressed(ord(']')):
      view.dbg_frame_advance = not view.dbg_frame_advance

  if view.dbg_frame_advance and not view.dbg_is_key_pressed(ord('\\')):
    glfw.swap_buffers(window)
    return

  style = ig.get_style()
  style.window_rounding = 0

  window_size = glfw.get_window_size(window)

  gl.glScissor(0, 0, *window_size)
  gl.glClearColor(0.06, 0.06, 0.06, 1.0)
  gl.glClear(gl.GL_COLOR_BUFFER_BIT)

  ig.get_style().colors[ig.COLOR_WINDOW_BACKGROUND] = (0, 0, 0, 0)
  ig.new_frame()
  view.render(window_size)
  ig.end_frame()
  ig.render()

  draw_data = ig.get_draw_data()
  ig_renderer.render(draw_data)

  glfw.swap_buffers(window)


def run() -> None:
  glfw.init()

  glfw.window_hint(glfw.CONTEXT_VERSION_MAJOR, 3)
  glfw.window_hint(glfw.CONTEXT_VERSION_MINOR, 3)
  glfw.window_hint(glfw.OPENGL_PROFILE, glfw.OPENGL_COMPAT_PROFILE) # TODO: Core
  glfw.window_hint(glfw.OPENGL_FORWARD_COMPAT, True)
  glfw.window_hint(glfw.SAMPLES, 4)

  glfw.window_hint(glfw.VISIBLE, False)
  window = glfw.create_window(800, 600, 'Wafel', None, None)
  glfw.set_window_size_limits(window, 1, 1, glfw.DONT_CARE, glfw.DONT_CARE)
  glfw.maximize_window(window)
  glfw.show_window(window)

  glfw.make_context_current(window)

  ig_context = ig.create_context()
  ig_renderer = GlfwRenderer(window)
  ig_renderer.io.ini_filename = None

  def refresh_callback(window):
    render(window, ig_renderer, view)
  glfw.set_window_refresh_callback(window, refresh_callback)

  model = Model()
  view = View(model)
  view.file = SequenceFile('test_files/1key_j.m64', 'm64')
  view.reload()
  view.file = None

  while not glfw.window_should_close(window):
    glfw.poll_events()
    ig_renderer.process_inputs()
    render(window, ig_renderer, view)
    model.timeline.balance_distribution(1/120)

  ig_renderer.shutdown()
  ig.destroy_context(ig_context)

  glfw.destroy_window(window)
  glfw.terminate()
