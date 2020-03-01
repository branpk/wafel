#include <cstdio>
#include <algorithm>

#include <pybind11/pybind11.h>
#include <glad.h>
#include <glm/glm.hpp>
namespace sm64 {
  typedef int8_t s8;
  typedef int16_t s16;
  typedef int32_t s32;
  typedef int64_t s64;
  typedef uint8_t u8;
  typedef uint16_t u16;
  typedef uint32_t u32;
  typedef uint64_t u64;
  typedef float f32;
  typedef double f64;

  enum ObjectList
  {
      OBJ_LIST_PLAYER,      //  (0) mario
      OBJ_LIST_UNUSED_1,    //  (1) (unused)
      OBJ_LIST_DESTRUCTIVE, //  (2) things that can be used to destroy other objects, like
                            //      bob-ombs and corkboxes
      OBJ_LIST_UNUSED_3,    //  (3) (unused)
      OBJ_LIST_GENACTOR,    //  (4) general actors. most normal 'enemies' or actors are
                            //      on this list. (MIPS, bullet bill, bully, etc)
      OBJ_LIST_PUSHABLE,    //  (5) pushable actors. This is a group of objects which
                            //      can push each other around as well as their parent
                            //      objects. (goombas, koopas, spinies)
      OBJ_LIST_LEVEL,       //  (6) level objects. general level objects such as heart, star
      OBJ_LIST_UNUSED_7,    //  (7) (unused)
      OBJ_LIST_DEFAULT,     //  (8) default objects. objects that didnt start with a 00
                            //      command are put here, so this is treated as a default.
      OBJ_LIST_SURFACE,     //  (9) surface objects. objects that specifically have surface
                            //      collision and not object collision. (thwomp, whomp, etc)
      OBJ_LIST_POLELIKE,    // (10) polelike objects. objects that attract or otherwise
                            //      "cling" mario similar to a pole action. (hoot,
                            //      whirlpool, trees/poles, etc)
      OBJ_LIST_SPAWNER,     // (11) spawners
      OBJ_LIST_UNIMPORTANT, // (12) unimportant objects. objects that will not load
                            //      if there are not enough object slots: they will also
                            //      be manually unloaded to make room for slots if the list
                            //      gets exhausted.
      NUM_OBJ_LISTS
  };

  typedef f32 Vec2f[2];
  typedef f32 Vec3f[3]; // X, Y, Z, where Y is up
  typedef s16 Vec3s[3];
  typedef s32 Vec3i[3];
  typedef f32 Vec4f[4];
  typedef s16 Vec4s[4];

  struct Surface
  {
      /*0x00*/ s16 type;
      /*0x02*/ s16 force;
      /*0x04*/ s8 flags;
      /*0x05*/ s8 room;
      /*0x06*/ s16 lowerY;
      /*0x08*/ s16 upperY;
      /*0x0A*/ Vec3s vertex1;
      /*0x10*/ Vec3s vertex2;
      /*0x16*/ Vec3s vertex3;
      /*0x1C*/ struct {
          f32 x;
          f32 y;
          f32 z;
      } normal;
      /*0x28*/ f32 originOffset;
      /*0x2C*/ struct Object *object;
  };

  struct MarioState
  {
      /*0x00*/ u16 unk00;
      /*0x02*/ u16 input;
      /*0x04*/ u32 flags;
      /*0x08*/ u32 particleFlags;
      /*0x0C*/ u32 action;
      /*0x10*/ u32 prevAction;
      /*0x14*/ u32 terrainSoundAddend;
      /*0x18*/ u16 actionState;
      /*0x1A*/ u16 actionTimer;
      /*0x1C*/ u32 actionArg;
      /*0x20*/ f32 intendedMag;
      /*0x24*/ s16 intendedYaw;
      /*0x26*/ s16 invincTimer;
      /*0x28*/ u8 framesSinceA;
      /*0x29*/ u8 framesSinceB;
      /*0x2A*/ u8 wallKickTimer;
      /*0x2B*/ u8 doubleJumpTimer;
      /*0x2C*/ Vec3s faceAngle;
      /*0x32*/ Vec3s angleVel;
      /*0x38*/ s16 slideYaw;
      /*0x3A*/ s16 twirlYaw;
      /*0x3C*/ Vec3f pos;
      /*0x48*/ Vec3f vel;
      /*0x54*/ f32 forwardVel;
      /*0x58*/ f32 slideVelX;
      /*0x5C*/ f32 slideVelZ;
      /*0x60*/ struct Surface *wall;
      /*0x64*/ struct Surface *ceil;
      /*0x68*/ struct Surface *floor;
      /*0x6C*/ f32 ceilHeight;
      /*0x70*/ f32 floorHeight;
      /*0x74*/ s16 floorAngle;
      /*0x76*/ s16 waterLevel;
      /*0x78*/ struct Object *interactObj;
      /*0x7C*/ struct Object *heldObj;
      /*0x80*/ struct Object *usedObj;
      /*0x84*/ struct Object *riddenObj;
      /*0x88*/ struct Object *marioObj;
      /*0x8C*/ struct SpawnInfo *spawnInfo;
      /*0x90*/ struct Area *area;
      /*0x94*/ struct PlayerCameraState *statusForCamera;
      /*0x98*/ struct MarioBodyState *marioBodyState;
      /*0x9C*/ struct Controller *controller;
      /*0xA0*/ struct MarioAnimation *animation;
      /*0xA4*/ u32 collidedObjInteractTypes;
      /*0xA8*/ s16 numCoins;
      /*0xAA*/ s16 numStars;
      /*0xAC*/ s8 numKeys; // Unused key mechanic
      /*0xAD*/ s8 numLives;
      /*0xAE*/ s16 health;
      /*0xB0*/ s16 unkB0;
      /*0xB2*/ u8 hurtCounter;
      /*0xB3*/ u8 healCounter;
      /*0xB4*/ u8 squishTimer;
      /*0xB5*/ u8 fadeWarpOpacity;
      /*0xB6*/ u16 capTimer;
      /*0xB8*/ s16 unkB8;
      /*0xBC*/ f32 peakHeight;
      /*0xC0*/ f32 quicksandDepth;
      /*0xC4*/ f32 unkC4;
  };
}
#include <glm/gtc/matrix_transform.hpp>

#include "renderer.hpp"
#include "util.hpp"

namespace py = pybind11;

using sm64::s8;
using sm64::s16;
using sm64::s32;
using sm64::s64;
using sm64::u8;
using sm64::u16;
using sm64::u32;
using sm64::u64;
using sm64::f32;
using sm64::f64;

typedef u64 uptr; // integer at least the size of a pointer, for pybind11 conversions


#define VEC3F_TO_VEC3(v) (vec3((v)[0], (v)[1], (v)[2]))


static uptr new_renderer() {
  static bool loaded_gl = false;

  if (!loaded_gl) {
    if (!gladLoadGL()) {
      throw std::runtime_error("Failed to load OpenGL");
    }
    loaded_gl = true;
  }

  Renderer *renderer = new Renderer;
  return (uptr) renderer;
}


static void delete_renderer(uptr renderer_addr) {
  Renderer *renderer = (Renderer *) renderer_addr;
  delete renderer;
}


// static void *segmented_to_virtual(sm64::SM64State *st, void *addr) {
//   void *result = ((void *)0);
//   s32 i = 0;
//   for (; (i < 32); (i++)) {
//     if (((st->sSegmentTable[i].srcStart <= addr) && (addr < st->sSegmentTable[i].srcEnd))) {
//       if ((result != ((void *)0))) {
//         fprintf(stderr, "Warning: segmented_to_virtual: Found two segments containing address\n");
//         exit(1);
//       }
//       (result = ((((u8 *)addr) - ((u8 *)st->sSegmentTable[i].srcStart)) + (u8 *)st->sSegmentTable[i].dstStart));
//     }
//   }
//   if ((result == ((void *)0))) {
//     (result = addr);
//   }
//   return result;
// }


// static u32 get_object_list_from_behavior(u32 *behavior) {
//   u32 objectList;

//   // If the first behavior command is "begin", then get the object list header
//   // from there
//   if ((behavior[0] >> 24) == 0) {
//     objectList = (behavior[0] >> 16) & 0xFFFF;
//   } else {
//     objectList = sm64::OBJ_LIST_DEFAULT;
//   }

//   return objectList;
// }

// static u32 get_object_list(sm64::Object *object) {
//   return get_object_list_from_behavior((u32 *) object->behavior);
// }


mat4 mat4_lookat(vec3 from, vec3 to, float roll) {
  float dx = to.x - from.x;
  float dz = to.z - from.z;

  float invLength = -1.0 / sqrtf(dx * dx + dz * dz);
  dx *= invLength;
  dz *= invLength;

  float yColY = cosf(roll);
  float xColY = sinf(roll) * dz;
  float zColY = -sinf(roll) * dx;

  float xColZ = to.x - from.x;
  float yColZ = to.y - from.y;
  float zColZ = to.z - from.z;

  invLength = -1.0 / sqrtf(xColZ * xColZ + yColZ * yColZ + zColZ * zColZ);
  xColZ *= invLength;
  yColZ *= invLength;
  zColZ *= invLength;

  float xColX = yColY * zColZ - zColY * yColZ;
  float yColX = zColY * xColZ - xColY * zColZ;
  float zColX = xColY * yColZ - yColY * xColZ;

  invLength = 1.0 / sqrtf(xColX * xColX + yColX * yColX + zColX * zColX);

  xColX *= invLength;
  yColX *= invLength;
  zColX *= invLength;

  xColY = yColZ * zColX - zColZ * yColX;
  yColY = zColZ * xColX - xColZ * zColX;
  zColY = xColZ * yColX - yColZ * xColX;

  invLength = 1.0 / sqrtf(xColY * xColY + yColY * yColY + zColY * zColY);
  xColY *= invLength;
  yColY *= invLength;
  zColY *= invLength;

  mat4 mtx;

  mtx[0][0] = xColX;
  mtx[1][0] = yColX;
  mtx[2][0] = zColX;
  mtx[3][0] = -(from[0] * xColX + from[1] * yColX + from[2] * zColX);

  mtx[0][1] = xColY;
  mtx[1][1] = yColY;
  mtx[2][1] = zColY;
  mtx[3][1] = -(from[0] * xColY + from[1] * yColY + from[2] * zColY);

  mtx[0][2] = xColZ;
  mtx[1][2] = yColZ;
  mtx[2][2] = zColZ;
  mtx[3][2] = -(from[0] * xColZ + from[1] * yColZ + from[2] * zColZ);

  mtx[0][3] = 0;
  mtx[1][3] = 0;
  mtx[2][3] = 0;
  mtx[3][3] = 1;

  return mtx;
}


struct GameState {
  py::object state_object;
  int frame;

  bool operator==(const GameState &other) const {
    return frame == other.frame;
  }

  py::object data(const std::string &path) const {
    return state_object.attr("get_data")(py::cast(path));
  }

  void *addr(const std::string &path) const {
    return (void *) state_object.attr("get_data_addr")(py::cast(path)).cast<uptr>();
  }
};


static vec3 read_vec3(py::object vec_object) {
  return vec3(
    vec_object[py::cast(0)].cast<float>(),
    vec_object[py::cast(1)].cast<float>(),
    vec_object[py::cast(2)].cast<float>());
}


static Viewport read_viewport(py::object viewport_object) {
  Viewport viewport = {
    ivec2(
      viewport_object.attr("x").cast<int>(),
      viewport_object.attr("y").cast<int>()),
    ivec2(
      viewport_object.attr("width").cast<int>(),
      viewport_object.attr("height").cast<int>()),
  };
  return viewport;
}


static Camera read_camera(py::object camera_object) {
  Camera camera;
  camera.mode = (CameraMode) camera_object.attr("mode").attr("value").cast<int>();

  switch (camera.mode) {
    case CameraMode::ROTATE: {
      RotateCamera rotate_camera = {
        read_vec3(camera_object.attr("pos")),
        camera_object.attr("pitch").cast<float>(),
        camera_object.attr("yaw").cast<float>(),
        camera_object.attr("fov_y").cast<float>(),
      };
      camera.rotate_camera = rotate_camera;
      break;
    }

    case CameraMode::BIRDS_EYE: {
      BirdsEyeCamera birds_eye_camera = {
        read_vec3(camera_object.attr("pos")),
        camera_object.attr("span_y").cast<float>(),
      };
      camera.birds_eye_camera = birds_eye_camera;
      break;
    }
  }

  return camera;
}


static GameState read_game_state(py::object state_object) {
  GameState state = {
    state_object,
    state_object.attr("frame").cast<int>(),
  };
  return state;
}


static vector<GameState> read_game_state_list(py::object states_object) {
  size_t length = py::len(states_object);
  vector<GameState> states = vector<GameState>(length);

  for (size_t i = 0; i < length; i++) {
    states[i] = read_game_state(states_object[py::cast(i)]);
  }

  return states;
}


float remove_x = 0;


static void render(uptr renderer_addr, py::object info) {
  Renderer *renderer = (Renderer *) renderer_addr;


  Scene scene;


  GameState st = read_game_state(info.attr("current_state"));


  // mat4 in_game_view_matrix;
  // {
  //   // f32 *camera_pos = st.data->D_8033B328.unk0[1];
  //   // f32 camera_pitch = st.data->D_8033B328.unk4C * 3.14159f / 0x8000;
  //   // f32 camera_yaw = st.data->D_8033B328.unk4E * 3.14159f / 0x8000;
  //   // f32 camera_roll = st.data->D_8033B328.unk7A * 3.14159f / 0x8000;
  //   // f32 camera_fov_y = st.data->D_8033B230.fieldOfView * 3.14159f / 180;

  //   vec3 camera_pos = VEC3F_TO_VEC3(st.data->D_8033B328.unk8C);
  //   vec3 camera_focus = VEC3F_TO_VEC3(st.data->D_8033B328.unk80);
  //   float camera_roll = st.data->D_8033B328.unk7A * glm::pi<float>() / 0x8000;

  //   in_game_view_matrix = mat4_lookat(camera_pos, camera_focus, camera_roll);

  //   // info->camera.mode = CameraMode::ROTATE;

  //   // info->camera.mode = CameraMode::ROTATE;
  //   // info->camera.rotate_camera = {
  //   //   VEC3F_TO_VEC3(camera_pos),
  //   //   camera_pitch,
  //   //   camera_yaw,
  //   //   camera_fov_y,
  //   // };
  // }


  scene.camera = read_camera(info.attr("camera"));


  s32 num_surfaces = st.data("$state.gSurfacesAllocated").cast<s32>();
  sm64::Surface *surfaces = (sm64::Surface *) st.data("$state.sSurfacePool").cast<uptr>();

  for (s32 i = 0; i < num_surfaces; i++) {
    struct sm64::Surface *surface = &surfaces[i];

    SurfaceType type;
    if (surface->normal.y > 0.01) {
      type = SurfaceType::FLOOR;
    } else if (surface->normal.y < -0.01) {
      type = SurfaceType::CEILING;
    } else if (surface->normal.x < -0.707 || surface->normal.x > 0.707) {
      type = SurfaceType::WALL_X_PROJ;
    } else {
      type = SurfaceType::WALL_Z_PROJ;
    }

    scene.surfaces.push_back({
      type,
      {
        vec3(surface->vertex1[0], surface->vertex1[1], surface->vertex1[2]),
        vec3(surface->vertex2[0], surface->vertex2[1], surface->vertex2[2]),
        vec3(surface->vertex3[0], surface->vertex3[1], surface->vertex3[2]),
      },
      vec3(surface->normal.x, surface->normal.y, surface->normal.z),
    });
  }

  // sm64::Object *object_pool = (sm64::Object *) st.addr("$state.gObjectPool");
  // for (s32 i = 0; i < 240; i++) {
  //   sm64::Object *obj = &object_pool[i];
  //   if (obj->activeFlags & ACTIVE_FLAG_ACTIVE) {
  //     scene.objects.push_back({
  //       vec3(obj->oPosX, obj->oPosY, obj->oPosZ),
  //       obj->hitboxHeight,
  //       obj->hitboxRadius,
  //     });
  //   }
  // }

  vector<GameState> path_states = read_game_state_list(info.attr("path_states"));

  size_t current_index = std::distance(
    path_states.begin(),
    std::find(path_states.begin(), path_states.end(), st));

  vector<ObjectPathNode> mario_path;
  for (GameState path_st : path_states) {
    sm64::MarioState *m = (sm64::MarioState *) path_st.data("$state.gMarioState").cast<uptr>();

    // if (!mario_path.empty() && mario_path.size() == current_index + 1) {
    //   sm64::QStepsInfo *qsteps = (sm64::QStepsInfo *) path_st.addr("$state.gQStepsInfo");
    //   if (qsteps->numSteps > 4) {
    //     printf("%d\n", qsteps->numSteps);
    //   }
    //   for (int i = 0; i < qsteps->numSteps; i++) {
    //     mario_path.back().quarter_steps.push_back({
    //       VEC3F_TO_VEC3(qsteps->steps[i].intendedPos),
    //       VEC3F_TO_VEC3(qsteps->steps[i].resultPos),
    //     });
    //   }
    // }

    mario_path.push_back({
      vec3(m->pos[0], m->pos[1], m->pos[2]),
      vector<QuarterStep>(),
    });
  }
  scene.object_paths.push_back({
    mario_path,
    current_index,
  });

  // for (s32 i = 0; i < 240; i++) {
  //   sm64::Object *obj = &st.data->gObjectPool[current_index];
  //   if (obj->activeFlags & ACTIVE_FLAG_ACTIVE) {
  //     vector<vec3> path;
  //     for (GameState path_st : info->path_states) {
  //       obj = &path_st.data->gObjectPool[i];
  //       path.push_back(vec3(obj->oPosX, obj->oPosY, obj->oPosZ));
  //     }
  //     scene.object_paths.push_back({
  //       path,
  //       current_index,
  //     });
  //   }
  // }

  renderer->render(read_viewport(info.attr("viewport")), scene);


  // glUseProgram(0);

  // // glClear(GL_COLOR_BUFFER_BIT | GL_DEPTH_BUFFER_BIT);
  // glViewport(viewport.pos.x, viewport.pos.y, viewport.size.x, viewport.size.y);

  // glEnable(GL_DEPTH_TEST);
  // glDepthFunc(GL_LEQUAL);

  // scene.camera = info->camera;
  // renderer->build_transforms(viewport, scene);

  // glMatrixMode(GL_PROJECTION);
  // glLoadMatrixf(&renderer->proj_matrix[0][0]);

  // glMatrixMode(GL_MODELVIEW);
  // glLoadMatrixf(&renderer->view_matrix[0][0]);
  // // glLoadIdentity();
  // // glTranslatef(0, 0, -100);
  // // float scale = 0.1f;
  // // glScalef(scale, scale, scale);

  // // glRotatef(remove_x, 0, 1, 0);
  // // remove_x += 1;

  // void interpret_display_list(GameState st, u32 *dl, string indent="");
  // mat4 matrix_fixed_to_float(u16 *mtx);

  // bool found = false;

  // // printf("%p %p, %p %p\n", st.base, st.base + 1, st.data, st.data + 1);

  // vec3 pos;

  // for (int i = 0; i < 8; i++) {
  //   sm64::GraphNodeToggleZBuffer_sub *node = st.from_base(st.data->gDisplayLists.unk14[i]);
  //   while (node != nullptr) {
  //     sm64::Object *object = st.from_base(node->object);
  //     sm64::Object *mario_object = st.from_base(st.data->gMarioObject);

  //     if (mario_object != nullptr && object == mario_object) {
  //       u16 *transform = (u16 *) st.from_base(node->unk0);
  //       // printf("%p -> %p\n", node->unk4, st.from_base(node->unk4));
  //       u32 *display_list = (u32 *) st.from_base(node->unk4);

  //       // printf("%d %p %08X %08X\n", i, display_list, display_list[0], display_list[1]);

  //       mat4 matrix = matrix_fixed_to_float(transform);
  //       matrix = glm::inverse(in_game_view_matrix) * matrix;



  //       // matrix = glm::inverse(renderer->view_matrix) * matrix;
  //       // if (!found) {
  //         // vec3 pos = vec3(matrix[3].x, matrix[3].y, matrix[3].z);
  //       //   // vec3 cam_pos = VEC3F_TO_VEC3(st.data->D_8033B328.unk0[1]);
  //         // printf("%f %f %f\n", pos.x, pos.y, pos.z);

  //         vec3 mario_pos = mario_path[current_index].pos;
  //         // vec3 mario_pos = VEC3F_TO_VEC3(st.from_base(st.data->gMarioObject)->header.gfx.pos);
  //         // matrix[3] -= vec4(mario_pos, 0);
  //         // matrix *= 1.0f;
  //         // matrix[3][3] = 1;
  //         // matrix[3] += vec4(mario_pos, 0);

  //         // matrix = glm::translate(mat4(1.0f), mario_pos);

  //       //   // printf("%f %f %f\n", cam_pos.x, cam_pos.y, cam_pos.z);
  //       //   // printf("\n");
  //       // }
  //       // matrix[3] = vec4(vec3(matrix[3].x, matrix[3].y, matrix[3].z) - pos, 1);
  //       // matrix[3] = vec4(0, 0, 0, 1);
  //       // for (int r = 0; r < 4; r++) {
  //       //   for (int c = 0; c < 4; c++) {
  //       //     printf("%f ", matrix[c][r]);
  //       //   }
  //       //   printf("\n");
  //       // }
  //       // printf("\n");
  //       // for (int r = 0; r < 4; r++) {
  //       //   for (int c = 0; c < 4; c++) {
  //       //     printf("%f ", renderer->view_matrix[c][r]);
  //       //   }
  //       //   printf("\n");
  //       // }
  //       // printf("pos = %f %f %f \n", mario_path[current_index].pos.x, mario_path[current_index].pos.y, mario_path[current_index].pos.z);
  //       // vec3 cam_pos = VEC3F_TO_VEC3(st.data->D_8033B328.unk0[1]);
  //       // printf("cam pos = %f %f %f\n", cam_pos.x, cam_pos.y, cam_pos.z);
  //       // // printf("\n");
  //       // printf("\n\n");

  //       glPushMatrix();
  //       glMultMatrixf(&matrix[0][0]);

  //       interpret_display_list(st, display_list);

  //       glPopMatrix();

  //       found = true;
  //     }

  //     node = st.from_base(node->unk8);
  //   }
  // }

  // // printf("\n");

  // // if (found) {
  // //   exit(0);
  // // }

  // // static bool done = false;
  // // if (!done) {
  // //   if (st.data->gMarioObject != NULL) {
  // //     sm64::Object *object = st.from_base(st.data->gMarioObject);
  // //     if (object != nullptr) {
  // //       printf("%p %p\n", object->displayListStart, object->displayListEnd);
  // //       // done = true;
  // //       // u32 *dl = (u32 *) object->displayListStart;
  // //       // while (dl < (u32 *) object->displayListEnd) {
  // //       //   printf("0x%08X\n", dl);
  // //       // }
  // //     }
  // //   }
  // // }
}


vector<vec3> loaded_vertices(32);


mat4 matrix_fixed_to_float(u16 *mtx) {
  mat4 result;
  for (size_t i = 0; i < 16; i++) {
    s32 val32 = (s32) ((mtx[i] << 16) + mtx[16 + i]);
    result[i / 4][i % 4] = (f32) val32 / 0x10000;
  }
  return result;
}


// void interpret_display_list(GameState st, u32 *dl, string indent) {
//   // printf("%s-----\n", indent.c_str());

//   while (true) {
//     u32 w0 = dl[0];
//     u32 w1 = dl[1];
//     u8 cmd = w0 >> 24;

//     // printf("%s%08X %08X\n", indent.c_str(), w0, w1);

//     switch (cmd) {
//     case 0x01: { // gSPMatrix
//       fprintf(stderr, "gSPMatrix\n");
//       exit(1);

//       // u8 p = (w0 >> 16) & 0xFF;
//       // u16 *fixed_point = st.from_base((u16 *) w1);
//       // mat4 matrix = matrix_fixed_to_float(fixed_point);

//       // glMatrixMode((p & 0x01) ? GL_PROJECTION : GL_MODELVIEW);

//       // if (p & 0x04) {
//       //   glPushMatrix();
//       // } else {
//       //   // no push
//       // }

//       // if (p & 0x02) {
//       //   // load
//       //   fprintf(stderr, "gSPMatrix load\n");
//       //   exit(1);
//       // } else {
//       //   glMultMatrixf(&matrix[0][0]);
//       // }

//       break;
//     }

//     case 0x03: // gSPViewport, gSPLight
//       break;

//     case 0x04: { // gSPVertex
//       u32 n = ((w0 >> 20) & 0xF) + 1;
//       u32 v0 = (w0 >> 16) & 0xF;
//       sm64::Vtx *v = st.from_base((sm64::Vtx *) w1);

//       loaded_vertices.clear();
//       for (u32 i = 0; i < n; i++) {
//         loaded_vertices[v0 + i] = vec3(v[i].v.ob[0], v[i].v.ob[1], v[i].v.ob[2]);
//       }

//       break;
//     }

//     case 0x06: { // gSPDisplayList, gSPBranchList
//       u32 *new_dl = st.from_base((u32 *) w1);
//       if (w0 == 0x06000000) {
//         interpret_display_list(st, new_dl, indent + "  ");
//       } else if (w0 == 0x06010000) {
//         dl = new_dl - 2;
//       } else {
//         fprintf(stderr, "gSPDisplayList: 0x%08X\n", w0);
//         exit(1);
//       }
//       break;
//     }

//     case 0xB6: // gSPClearGeometryMode
//       break;

//     case 0xB7: // gSPSetGeometryMode
//       break;

//     case 0xB8: // gSPEndDisplayList
//       return;

//     case 0xB9: // gDPSetAlphaCompare, gDPSetDepthSource, gDPSetRenderMode
//       break;

//     case 0xBB: // gSPTexture
//       break;

//     case 0xBF: { // gSP1Triangle
//       u32 v0 = ((w1 >> 16) & 0xFF) / 10;
//       u32 v1 = ((w1 >> 8) & 0xFF) / 10;
//       u32 v2 = ((w1 >> 0) & 0xFF) / 10;

//       glBegin(GL_LINE_LOOP);
//       glVertex3f(loaded_vertices[v0].x, loaded_vertices[v0].y, loaded_vertices[v0].z);
//       glVertex3f(loaded_vertices[v1].x, loaded_vertices[v1].y, loaded_vertices[v1].z);
//       glVertex3f(loaded_vertices[v2].x, loaded_vertices[v2].y, loaded_vertices[v2].z);
//       glEnd();

//       break;
//     }

//     case 0xE6: // gDPLoadSync
//       break;

//     case 0xE7: // gDPPipeSync
//       break;

//     case 0xE8: // gDPTileSync
//       break;

//     case 0xF2: // gDPSetTileSize
//       break;

//     case 0xF3: // gDPLoadBlock
//       break;

//     case 0xF5: // gDPSetTile
//       break;

//     case 0xFB: // gDPSetEnvColor
//       break;

//     case 0xFC: // gDPSetCombineMode
//       break;

//     case 0xFD: // gDPSetTextureImage
//       break;

//     default:
//       // fprintf(stderr, "0x%02X\n", cmd);
//       // exit(1);
//       break;
//     }

//     dl += 2;
//   }
// }


PYBIND11_MODULE(graphics, m) {
  m.def("new_renderer", new_renderer);
  m.def("delete_renderer", delete_renderer);
  m.def("render", render);
}
