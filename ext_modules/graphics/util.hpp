#ifndef _GRAPHICS_UTIL_HPP
#define _GRAPHICS_UTIL_HPP

#define VEC_SIZE(v) ((v).size() * sizeof((v)[0]))

#include <vector>
#include <string>
#include <utility>
#include <map>

#include <glm/vec2.hpp>
#include <glm/vec3.hpp>
#include <glm/vec4.hpp>
#include <glm/mat4x4.hpp>
#include <glad.h>

using std::vector;
using std::string;
using std::pair;
using std::map;

using glm::ivec2;
using glm::vec2;
using glm::vec3;
using glm::vec4;
using glm::mat4;

#endif
