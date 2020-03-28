#include <functional>
#include <utility>
#include <limits>

#include <pybind11/pybind11.h>
#include <pybind11/functional.h>
#include <pybind11/stl.h>

#include "sm64_types.hpp"

namespace py = pybind11;
using namespace std;


static s16 (*atan2s)(f32 a, f32 b);
static f32 *gSineTable;

#define gCosineTable (gSineTable + 0x400)

#define sins(x) gSineTable[(u16) (x) >> 4]
#define coss(x) gCosineTable[(u16) (x) >> 4]


static void init_adjusted_yaw_table();


static void init(function<uintptr_t(string)> get_static_addr) {
  atan2s = (decltype(atan2s)) get_static_addr("atan2s");
  gSineTable = (decltype(gSineTable)) get_static_addr("gSineTable");

  init_adjusted_yaw_table();
}


struct AdjustedStick {
  f32 x;
  f32 y;
  f32 mag;
};


static AdjustedStick stick_raw_to_adjusted(s16 raw_stick_x, s16 raw_stick_y) {
  AdjustedStick stick;

  stick.x = 0;
  stick.y = 0;

  if (raw_stick_x <= -8) {
    stick.x = raw_stick_x + 6;
  }
  if (raw_stick_x >= 8) {
    stick.x = raw_stick_x - 6;
  }
  if (raw_stick_y <= -8) {
    stick.y = raw_stick_y + 6;
  }
  if (raw_stick_y >= 8) {
    stick.y = raw_stick_y - 6;
  }

  stick.mag = sqrtf(stick.x * stick.x + stick.y * stick.y);

  if (stick.mag > 64) {
    stick.x *= 64 / stick.mag;
    stick.y *= 64 / stick.mag;
    stick.mag = 64;
  }

  return stick;
}


static pair<s16, f32> stick_adjusted_to_intended(
  AdjustedStick stick, s16 face_yaw, s16 camera_yaw, bool squished)
{
  s16 intended_yaw;
  f32 intended_mag;

  f32 mag = ((stick.mag / 64.0f) * (stick.mag / 64.0f)) * 64.0f;

  if (!squished) {
    intended_mag = mag / 2.0f;
  } else {
    intended_mag = mag / 8.0f;
  }

  if (intended_mag > 0.0f) {
    intended_yaw = atan2s(-stick.y, stick.x) + camera_yaw;
  } else {
    intended_yaw = face_yaw;
  }

  return make_pair(intended_yaw, intended_mag);
}


static pair<s16, s16> adjusted_yaw_table[0x10000];


static void init_adjusted_yaw_table() {
  for (s16 raw_stick_x = -128; raw_stick_x <= 127; raw_stick_x++) {
    for (s16 raw_stick_y = -128; raw_stick_y <= 127; raw_stick_y++) {
      AdjustedStick stick = stick_raw_to_adjusted(raw_stick_x, raw_stick_y);
      if (stick.mag >= 64.0f) {
        u16 adjusted_yaw = (u16) atan2s(-stick.y, stick.x);
        adjusted_yaw_table[adjusted_yaw] = make_pair(raw_stick_x, raw_stick_y);
      }
    }
  }
}


static pair<s16, s16> stick_adjusted_yaw_range_to_raw(s16 min_yaw, s16 max_yaw) {
  for (s16 yaw = min_yaw; yaw != max_yaw; yaw = (s16) ((u16) yaw + 1)) {
    pair<s16, s16> stick = adjusted_yaw_table[(u16) yaw];
    if (stick.first != 0 || stick.second != 0) {
      return stick;
    }
  }
  return make_pair(0, 0);
}


static pair<s16, s16> stick_adjusted_yaw_to_raw(s16 target_yaw, s16 relative_to) {
  u16 target_hau = (u16) (target_yaw - relative_to) >> 4;

  s16 distance = 0;
  while (true) {
    s16 min_yaw = (s16) (relative_to + ((u16) (target_hau + distance) << 4));

    pair<s16, s16> stick = stick_adjusted_yaw_range_to_raw(min_yaw, min_yaw + 16);
    if (stick.first != 0 || stick.second != 0) {
      return stick;
    }

    distance = -distance;
    if (distance >= 0) {
      distance += 1;
    }
  }
}


// Only valid for stick.mag < 64
static pair<s16, s16> stick_adjusted_to_raw_approx(AdjustedStick stick) {
  s16 raw_stick_x = 0;
  s16 raw_stick_y = 0;

  if (stick.x <= -2) {
    raw_stick_x = (s16) (stick.x - 6);
  }
  if (stick.x >= 2) {
    raw_stick_x = (s16) (stick.x + 6);
  }
  if (stick.y <= -2) {
    raw_stick_y = (s16) (stick.y - 6);
  }
  if (stick.y >= 2) {
    raw_stick_y = (s16) (stick.y + 6);
  }

  return make_pair(raw_stick_x, raw_stick_y);
}


static AdjustedStick stick_intended_to_adjusted_approx(
  s16 intended_yaw, f32 intended_mag, s16 face_yaw, s16 camera_yaw, bool squished)
{
  f32 mag;
  if (!squished) {
    mag = intended_mag * 2.0f;
  } else {
    mag = intended_mag * 8.0f;
  }

  AdjustedStick stick;
  stick.mag = sqrtf(mag / 64.0f) * 64.0f;
  stick.x = (s16) roundf(sins(intended_yaw - camera_yaw) * stick.mag);
  stick.y = (s16) roundf(-coss(intended_yaw - camera_yaw) * stick.mag);

  return stick;
}


template <typename T>
static pair<s16, s16> raw_joystick_min(function<T(s16, s16)> get_value) {
  pair<s16, s16> best(0, 0);
  T min_value = get_value(best.first, best.second);

  for (s16 x = -128; x <= 127; x++) {
    for (s16 y = -128; y <= 127; y++) {
      T value = get_value(x, y);
      if (value < min_value) {
        best = make_pair(x, y);
        min_value = value;
      }
    }
  }

  return best;
}


template <typename T>
static pair<s16, s16> raw_joystick_min_rect(
  s16 min_x,
  s16 max_x,
  s16 min_y,
  s16 max_y,
  function<T(s16, s16)> get_value)
{
  min_x = min(max(min_x, -128), 127);
  max_x = min(max(max_x, -128), 127);
  min_y = min(max(min_y, -128), 127);
  max_y = min(max(max_y, -128), 127);

  pair<s16, s16> best(0, 0);
  T min_value = get_value(best.first, best.second);

  for (s16 x = min_x; x <= max_x; x++) {
    for (s16 y = min_y; y <= max_y; y++) {
      T value = get_value(x, y);
      if (value < min_value) {
        best = make_pair(x, y);
        min_value = value;
      }
    }
  }

  return best;
}


static pair<s16, s16> stick_adjusted_to_raw(f32 target_x, f32 target_y) {
  return raw_joystick_min<f32>([&](s16 x, s16 y) {
    AdjustedStick stick = stick_raw_to_adjusted(x, y);
    f32 dx = stick.x - target_x;
    f32 dy = stick.y - target_y;
    return dx * dx + dy * dy;
  });
}


static pair<s16, s16> stick_intended_to_raw_exact(
  s16 target_yaw,
  f32 target_mag,
  s16 face_yaw,
  s16 camera_yaw,
  bool squished,
  s16 relative_to)
{
  return raw_joystick_min<pair<s32, f32>>([&](s16 x, s16 y) {
    pair<s16, f32> intended = stick_adjusted_to_intended(
      stick_raw_to_adjusted(x, y),
      face_yaw,
      camera_yaw,
      squished);
    s16 intended_yaw = intended.first;
    f32 intended_mag = intended.second;

    return make_pair(
      abs((u16) (target_yaw - relative_to) / 16 - (u16) (intended_yaw - relative_to) / 16),
      fabs(target_mag - intended_mag));
  });
}


static pair<s16, s16> stick_intended_to_raw(
  s16 target_yaw,
  f32 target_mag,
  s16 face_yaw,
  s16 camera_yaw,
  bool squished,
  s16 relative_to)
{
  AdjustedStick stick = stick_intended_to_adjusted_approx(
    target_yaw, target_mag, face_yaw, camera_yaw, squished
  );

  if (stick.mag >= 64.0f) {
    return stick_adjusted_yaw_to_raw(target_yaw - camera_yaw, relative_to - camera_yaw);
  } else {
    // TODO: Use raw_joystick_min_rect
    return stick_adjusted_to_raw_approx(stick);
  }

  // f32 target_s = target_mag * sins(target_yaw);
  // f32 target_c = target_mag * coss(target_yaw);

  // return raw_joystick_min<f32>([&](s16 x, s16 y) {
  //   pair<s16, f32> intended = stick_adjusted_to_intended(
  //     stick_raw_to_adjusted(x, y),
  //     face_yaw,
  //     camera_yaw,
  //     squished);
  //   s16 intended_yaw = intended.first;
  //   f32 intended_mag = intended.second;

  //   f32 s = intended_mag * sins(intended_yaw);
  //   f32 c = intended_mag * coss(intended_yaw);

  //   f32 ds = s - target_s;
  //   f32 dc = c - target_c;
  //   return ds * ds + dc * dc;
  // });
}


PYBIND11_MODULE(util, m) {
  m.def("init", init);
  m.def("stick_raw_to_adjusted", stick_raw_to_adjusted);
  m.def("stick_adjusted_to_intended", stick_adjusted_to_intended);
  m.def("stick_adjusted_to_raw", stick_adjusted_to_raw);
  m.def("stick_intended_to_raw", stick_intended_to_raw);

  py::class_<AdjustedStick>(m, "AdjustedStick")
    .def_readonly("x", &AdjustedStick::x)
    .def_readonly("y", &AdjustedStick::y)
    .def_readonly("mag", &AdjustedStick::mag);
}
