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


import os
from ctypes import *
import json
import time


def read_byte(f):
  return (f.read(1) or [0])[0]


def run():
  lib = cdll.LoadLibrary('lib/sm64plus/us/sm64plus')
  with open('lib/sm64plus/us/sm64plus.json', 'r') as f:
    spec = json.load(f)

  lib.sm64_state_new.argtypes = []
  lib.sm64_state_new.restype = c_void_p

  lib.sm64_state_delete.argtypes = [c_void_p]
  lib.sm64_state_delete.restype = None

  lib.sm64_state_update.argtypes = [c_void_p]
  lib.sm64_state_update.restype = None

  with open('test_files/120_u.m64', 'rb') as m64:
    m64.seek(0x400)

    st = lib.sm64_state_new()

    globals = spec['types']['struct']['SM64State']['fields']

    controller = st + globals['gControllerPads']['offset']
    os_cont_pad = spec['types']['typedef']['OSContPad']['fields']
    controller_button = cast(controller + os_cont_pad['button']['offset'], POINTER(c_uint16))
    controller_stick_x = cast(controller + os_cont_pad['stick_x']['offset'], POINTER(c_int8))
    controller_stick_y = cast(controller + os_cont_pad['stick_y']['offset'], POINTER(c_int8))

    global_timer = cast(st + globals['gGlobalTimer']['offset'], POINTER(c_uint32))
    level_num = cast(st + globals['gCurrLevelNum']['offset'], POINTER(c_int16))
    num_stars = cast(st + globals['gDisplayedStars']['offset'], POINTER(c_int16))

    start = time.time()

    for _ in range(150000):
      controller_button[0] = read_byte(m64) << 8 | read_byte(m64)
      controller_stick_x[0] = read_byte(m64)
      controller_stick_y[0] = read_byte(m64)

      lib.sm64_state_update(st)

      if global_timer[0] % 5000 == 0:
        print(num_stars[0])

    print(global_timer[0] / (time.time() - start))

  lib.sm64_state_delete(st)
