# import sys

# from PyQt5.QtCore import *
# from PyQt5.QtGui import *
# from PyQt5.QtWidgets import *


# class Window(QWidget):
#   def __init__(self, parent=None):
#     super().__init__(parent)


# class Model(QAbstractTableModel):
#   def __init__(self, parent=None):
#     super().__init__(parent)
#     self.data = {}

#   def rowCount(self, parent=None):
#     return 100

#   def columnCount(self, parent=None):
#     return 5

#   def flags(self, index):
#     return Qt.ItemIsSelectable | Qt.ItemIsEditable | Qt.ItemIsEnabled

#   def data(self, index, role=Qt.DisplayRole):
#     if role == Qt.DisplayRole or role == Qt.EditRole:
#       sys.stdout.write(str(index.row()) + ' ')
#       sys.stdout.flush()
#       return f'{self.data.get((index.row(), index.column())) or 0}'

#   def setData(self, index, value, role=Qt.EditRole):
#     # print(f'{index.row()}, {index.column()} = {value}')
#     self.data[(index.row(), index.column())] = value
#     self.dataChanged.emit(index, index)
#     return True


# def run():
#   app = QApplication(sys.argv)

#   # window = Window()
#   # window.show()

#   model = Model()

#   table = QTableView()
#   table.setModel(model)
#   table.setMinimumWidth(640)
#   table.setMinimumHeight(480)
#   table.setVerticalScrollMode(QAbstractItemView.ScrollPerPixel)
#   table.setHorizontalScrollMode(QAbstractItemView.ScrollPerPixel)

#   def showBounds():
#     sys.stdout.write('\n')
#     rect = table.contentsRect()
#     # print(f'{table.rowAt(rect.y())}, {table.rowAt(rect.y() + rect.height())}')

#   timer = QTimer()
#   timer.timeout.connect(showBounds)
#   timer.start(100)

#   def foo():
#     print('CHANGING')
#     model.setData(model.index(5, 0), 3)
#   timer2 = QTimer()
#   timer2.timeout.connect(foo)
#   timer2.start(5000)

#   table.show()

#   sys.exit(app.exec_())


# import os
# import ctypes
# import json
# import time

# from butter.util import dcast


# # Cell = allocated memory block
# # State = frame index + assigned cell containing up-to-date data

# # Maintain a list of free cells, and a list of states
# # Allocate one cell as the base cell, and one cell as the temp cell
# # Create a power-on state and assign the base cell to it
# # Keep track of the loaded state, which is the one associated with the base cell

# # To modify a state, modify its cell and then delete all later states

# # To load a state:
# #   Check that it's not already loaded
# #   Use the temp cell to swap the data in the base cell and the state's cell
# #   Swap the base cell and the state's cell

# # To update a state:
# #   Load it if it's not already
# #   Update the base cell
# #   Increment the state's frame index

# # To allocate a state:
# #   If a cell is free, use that
# #   Otherwise use an algorithm to pick a state - never the power on state
# #   Create a new state object that is a copy of the selected one
# #   Invalidate the selected state object (in case anyone has a reference to it)

# # To request a frame:
# #   Find the latest state earlier than it
# #   Allocate a new state
# #   If the new state is not sufficient, copy from the latest state
# #   Update the state to the desired frame
# #
# #   Performance:
# #     Copy latest -> new if new is too late or too early
# #     Swap with base (3 copies) if new is not allocated from the base cell
# #     So faster to allocate from base cell, but want to maintain a healthy
# #       distribution of states across time



# from ctypes import *

# from butter.state import State, StateManager


# def read_byte(f):
#   return (f.read(1) or [0])[0]


# def run():
#   lib = cdll.LoadLibrary('lib/sm64plus/us/sm64plus')
#   with open('lib/sm64plus/us/sm64plus.json', 'r') as f:
#     spec = json.load(f)

#   lib.sm64_state_new.argtypes = []
#   lib.sm64_state_new.restype = c_void_p

#   lib.sm64_state_delete.argtypes = [c_void_p]
#   lib.sm64_state_delete.restype = None

#   lib.sm64_state_update.argtypes = [c_void_p]
#   lib.sm64_state_update.restype = None

#   lib.sm64_state_raw_copy.argtypes = [c_void_p, c_void_p]
#   lib.sm64_state_raw_copy.restype = None

#   with open('test_files/120_u.m64', 'rb') as m64:
#     m64.seek(0x400)

#     state_manager = StateManager(lib, spec, 10)
#     st = state_manager.new_state(0)
#     state_manager._load_state(st)

#     globals = spec['types']['struct']['SM64State']['fields']

#     start = time.time()

#     for _ in range(150000):
#       controller = st.addr + globals['gControllerPads']['offset']
#       os_cont_pad = spec['types']['typedef']['OSContPad']['fields']
#       controller_button = cast(controller + os_cont_pad['button']['offset'], POINTER(c_uint16))
#       controller_stick_x = cast(controller + os_cont_pad['stick_x']['offset'], POINTER(c_int8))
#       controller_stick_y = cast(controller + os_cont_pad['stick_y']['offset'], POINTER(c_int8))

#       global_timer = cast(st.addr + globals['gGlobalTimer']['offset'], POINTER(c_uint32))
#       level_num = cast(st.addr + globals['gCurrLevelNum']['offset'], POINTER(c_int16))
#       num_stars = cast(st.addr + globals['gDisplayedStars']['offset'], POINTER(c_int16))

#       controller_button[0] = read_byte(m64) << 8 | read_byte(m64)
#       controller_stick_x[0] = read_byte(m64)
#       controller_stick_y[0] = read_byte(m64)
#       st.touch()

#       st.advance()

#       if global_timer[0] % 5000 == 0:
#         print(num_stars[0])

#     print(st.frame / (time.time() - start))


from typing import cast
from ctypes import *
import json

from PyQt5.QtWidgets import *
from PyQt5.QtCore import *
from PyQt5.QtGui import *

import butter.graphics as graphics
from butter.game_state import GameStateManager, GameState, InputSequence


from typing import BinaryIO
def read_byte(f: BinaryIO):
  return (f.read(1) or [0])[0]


class Window(QWidget):

  def __init__(self, parent=None):
    super().__init__(parent)

    self.setWindowTitle('SM64')

    layout = QVBoxLayout()
    layout.setContentsMargins(0, 0, 0, 0)

    self.game_view = GameView()
    layout.addWidget(self.game_view)

    self.slider = FrameSlider(150000, self.game_view.state_manager)
    def slider_value_changed(value):
      self.game_view.set_frame(value)
    self.slider.valueChanged.connect(slider_value_changed)
    layout.addWidget(self.slider)

    self.setLayout(layout)

    self.draw_timer = QTimer()
    self.draw_timer.timeout.connect(lambda: self.game_view.update())
    self.draw_timer.start()


class FrameSlider(QSlider):

  def __init__(self, length: int, state_manager: GameStateManager, parent=None):
    super().__init__(Qt.Horizontal, parent=parent)
    self.length = length
    self.state_manager = state_manager

    self.setMinimum(0)
    self.setMaximum(self.length)

  def paintEvent(self, event):
    super().paintEvent(event)

    painter = QPainter(self)
    for frame in self.state_manager.get_loaded_frames():
      x = (self.contentsRect().width() - 11) / self.length * frame + 5
      painter.fillRect(x, 0, 1, 20, Qt.red)


class GameView(QOpenGLWidget):

  def __init__(self, parent=None):
    super().__init__(parent)

    self.setMinimumSize(640, 480)

    with open('test_files/120_u.m64', 'rb') as m64:
      inputs = InputSequence.from_m64(m64)

    lib = cdll.LoadLibrary('lib/sm64plus/us/sm64plus')
    with open('lib/sm64plus/us/sm64plus.json', 'r') as f:
      self.spec = json.load(f)
    self.state_manager = GameStateManager(lib, self.spec, inputs, 200)

    self.frame = 0

  def set_frame(self, frame):
    self.frame = frame

  def initializeGL(self):
    graphics.load_gl()

  def paintGL(self):
    self.makeCurrent()

    st = self.state_manager.request_frame(self.frame)
    graphics.render(st)


def run():
  app = QApplication([])
  window = Window()
  window.show()
  app.exec_()
