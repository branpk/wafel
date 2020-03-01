#ifndef _SM64_H
#define _SM64_H

#if UINTPTR_MAX == 0xFFFFFFFFFFFFFFFF
#define IS_64_BIT 1
#endif

#include <stdint.h>

// TODO: Look up offsets dynamically instead of this header?

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

#define ACTIVE_FLAG_ACTIVE                 (1 <<  0) // 0x0001
#define ACTIVE_FLAG_FAR_AWAY               (1 <<  1) // 0x0002
#define ACTIVE_FLAG_UNK2                   (1 <<  2) // 0x0004
#define ACTIVE_FLAG_IN_DIFFERENT_ROOM      (1 <<  3) // 0x0008
#define ACTIVE_FLAG_UNIMPORTANT            (1 <<  4) // 0x0010
#define ACTIVE_FLAG_INITIATED_TIME_STOP    (1 <<  5) // 0x0020
#define ACTIVE_FLAG_MOVE_THROUGH_GRATE     (1 <<  6) // 0x0040
#define ACTIVE_FLAG_UNK7                   (1 <<  7) // 0x0080
#define ACTIVE_FLAG_UNK8                   (1 <<  8) // 0x0100
#define ACTIVE_FLAG_UNK9                   (1 <<  9) // 0x0200
#define ACTIVE_FLAG_UNK10                  (1 << 10) // 0x0400

#define ACTIVE_FLAGS_DEACTIVATED 0

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

typedef f32 Mat4[4][4];

typedef uintptr_t GeoLayout;
typedef uintptr_t LevelScript;
typedef s16 Movtex;
typedef s16 MacroObject;
typedef s16 Collision;
typedef s16 Trajectory;
typedef s16 PaintingData;
typedef uintptr_t BehaviorScript;

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

struct GraphNode
{
    /*0x00*/ s16 type; // structure type
    /*0x02*/ s16 flags; // hi = drawing layer, lo = rendering modes
    /*0x04*/ struct GraphNode *prev;
    /*0x08*/ struct GraphNode *next;
    /*0x0C*/ struct GraphNode *parent;
    /*0x10*/ struct GraphNode *children;
};

// struct AnimInfo?
struct GraphNodeObject_sub
{
    /*0x00 0x38*/ s16 animID;
    /*0x02 0x3A*/ s16 animYTrans;
    /*0x04 0x3C*/ struct Animation *curAnim;
    /*0x08 0x40*/ s16 animFrame;
    /*0x0A 0x42*/ u16 animTimer;
    /*0x0C 0x44*/ s32 animFrameAccelAssist;
    /*0x10 0x48*/ s32 animAccel;
};

struct GraphNodeObject
{
    /*0x00*/ struct GraphNode node;
    /*0x14*/ struct GraphNode *sharedChild;
    /*0x18*/ s8 unk18;
    /*0x19*/ s8 unk19;
    /*0x1A*/ Vec3s angle;
    /*0x20*/ Vec3f pos;
    /*0x2C*/ Vec3f scale;
    /*0x38*/ struct GraphNodeObject_sub unk38;
    /*0x4C*/ struct SpawnInfo *unk4C;
    /*0x50*/ void *throwMatrix; // matrix ptr
    /*0x54*/ Vec3f cameraToObject;
};

struct ObjectNode
{
    struct GraphNodeObject gfx;
    struct ObjectNode *next;
    struct ObjectNode *prev;
};

struct Object
{
    /*0x000*/ struct ObjectNode header;
    /*0x068*/ struct Object *parentObj;
    /*0x06C*/ struct Object *prevObj;
    /*0x070*/ u32 collidedObjInteractTypes;
    /*0x074*/ s16 activeFlags;
    /*0x076*/ s16 numCollidedObjs;
    /*0x078*/ struct Object *collidedObjs[4];
    /*0x088*/
    union
    {
        // Object fields. See object_fields.h.
        u32 asU32[0x50];
        s32 asS32[0x50];
        s16 asS16[0x50][2];
        f32 asF32[0x50];
#if !IS_64_BIT
        s16 *asS16P[0x50];
        s32 *asS32P[0x50];
        struct Animation **asAnims[0x50];
        struct Waypoint *asWaypoint[0x50];
        struct ChainSegment *asChainSegment[0x50];
        struct Object *asObject[0x50];
        struct Surface *asSurface[0x50];
        void *asVoidPtr[0x50];
        const void *asConstVoidPtr[0x50];
#endif
    } rawData;
#if IS_64_BIT
    union {
        s16 *asS16P[0x50];
        s32 *asS32P[0x50];
        struct Animation **asAnims[0x50];
        struct Waypoint *asWaypoint[0x50];
        struct ChainSegment *asChainSegment[0x50];
        struct Object *asObject[0x50];
        struct Surface *asSurface[0x50];
        void *asVoidPtr[0x50];
        const void *asConstVoidPtr[0x50];
    } ptrData;
#endif
    /*0x1C8*/ u32 unused1;
    /*0x1CC*/ const BehaviorScript *behScript;
    /*0x1D0*/ u32 stackIndex;
    /*0x1D4*/ uintptr_t stack[8];
    /*0x1F4*/ s16 unk1F4;
    /*0x1F6*/ s16 respawnInfoType;
    /*0x1F8*/ f32 hitboxRadius;
    /*0x1FC*/ f32 hitboxHeight;
    /*0x200*/ f32 hurtboxRadius;
    /*0x204*/ f32 hurtboxHeight;
    /*0x208*/ f32 hitboxDownOffset;
    /*0x20C*/ const BehaviorScript *behavior;
    /*0x210*/ u32 unused2;
    /*0x214*/ struct Object *platform;
    /*0x218*/ void *collisionData;
    /*0x21C*/ Mat4 transform;
    /*0x25C*/ void *respawnInfo;
};

enum QStepType {
    QSTEP_TYPE_NONE,
    QSTEP_TYPE_AIR,
    QSTEP_TYPE_GROUND,
};

struct QStepInfo {
    Vec3f startPos;
    Vec3f intendedPos;
    Vec3f resultPos;
    s32 event;
};

struct QStepsInfo {
    s32 type;
    s32 numSteps;
    struct QStepInfo steps[4];
};

#ifdef OBJECT_FIELDS_INDEX_DIRECTLY
#define OBJECT_FIELD_U32(index)           index
#define OBJECT_FIELD_S32(index)           index
#define OBJECT_FIELD_S16(index, subIndex) index
#define OBJECT_FIELD_F32(index)           index
#define OBJECT_FIELD_S16P(index)          index
#define OBJECT_FIELD_S32P(index)          index
#define OBJECT_FIELD_ANIMS(index)         index
#define OBJECT_FIELD_WAYPOINT(index)      index
#define OBJECT_FIELD_CHAIN_SEGMENT(index) index
#define OBJECT_FIELD_OBJ(index)           index
#define OBJECT_FIELD_SURFACE(index)       index
#define OBJECT_FIELD_VPTR(index)          index
#define OBJECT_FIELD_CVPTR(index)         index
#else
#define OBJECT_FIELD_U32(index)           rawData.asU32[index]
#define OBJECT_FIELD_S32(index)           rawData.asS32[index]
#define OBJECT_FIELD_S16(index, subIndex) rawData.asS16[index][subIndex]
#define OBJECT_FIELD_F32(index)           rawData.asF32[index]
#if !IS_64_BIT
#define OBJECT_FIELD_S16P(index)          rawData.asS16P[index]
#define OBJECT_FIELD_S32P(index)          rawData.asS32P[index]
#define OBJECT_FIELD_ANIMS(index)         rawData.asAnims[index]
#define OBJECT_FIELD_WAYPOINT(index)      rawData.asWaypoint[index]
#define OBJECT_FIELD_CHAIN_SEGMENT(index) rawData.asChainSegment[index]
#define OBJECT_FIELD_OBJ(index)           rawData.asObject[index]
#define OBJECT_FIELD_SURFACE(index)       rawData.asSurface[index]
#define OBJECT_FIELD_VPTR(index)          rawData.asVoidPtr[index]
#define OBJECT_FIELD_CVPTR(index)         rawData.asConstVoidPtr[index]
#else
#define OBJECT_FIELD_S16P(index)          ptrData.asS16P[index]
#define OBJECT_FIELD_S32P(index)          ptrData.asS32P[index]
#define OBJECT_FIELD_ANIMS(index)         ptrData.asAnims[index]
#define OBJECT_FIELD_WAYPOINT(index)      ptrData.asWaypoint[index]
#define OBJECT_FIELD_CHAIN_SEGMENT(index) ptrData.asChainSegment[index]
#define OBJECT_FIELD_OBJ(index)           ptrData.asObject[index]
#define OBJECT_FIELD_SURFACE(index)       ptrData.asSurface[index]
#define OBJECT_FIELD_VPTR(index)          ptrData.asVoidPtr[index]
#define OBJECT_FIELD_CVPTR(index)         ptrData.asConstVoidPtr[index]
#endif
#endif

// 0x088 (0x00), the first field, is object-specific and defined below the common fields.
/* Common fields */
#define /*0x08C*/ oFlags                      OBJECT_FIELD_U32(0x01)
#define /*0x090*/ oDialogResponse             OBJECT_FIELD_S16(0x02, 0)
#define /*0x092*/ oDialogState                OBJECT_FIELD_S16(0x02, 1)
#define /*0x094*/ oUnk94                      OBJECT_FIELD_U32(0x03)
// 0x98 unused/removed.
#define /*0x09C*/ oIntangibleTimer            OBJECT_FIELD_S32(0x05)
#define /*0x0A0*/ O_POS_INDEX                 0x06
#define /*0x0A0*/ oPosX                       OBJECT_FIELD_F32(O_POS_INDEX + 0)
#define /*0x0A4*/ oPosY                       OBJECT_FIELD_F32(O_POS_INDEX + 1)
#define /*0x0A8*/ oPosZ                       OBJECT_FIELD_F32(O_POS_INDEX + 2)
#define /*0x0AC*/ oVelX                       OBJECT_FIELD_F32(0x09)
#define /*0x0B0*/ oVelY                       OBJECT_FIELD_F32(0x0A)
#define /*0x0B4*/ oVelZ                       OBJECT_FIELD_F32(0x0B)
#define /*0x0B8*/ oForwardVel                 OBJECT_FIELD_F32(0x0C)
#define /*0x0B8*/ oForwardVelS32              OBJECT_FIELD_S32(0x0C)
#define /*0x0BC*/ oUnkBC                      OBJECT_FIELD_F32(0x0D)
#define /*0x0C0*/ oUnkC0                      OBJECT_FIELD_F32(0x0E)
#define /*0x0C4*/ O_MOVE_ANGLE_INDEX          0x0F
#define /*0x0C4*/ O_MOVE_ANGLE_PITCH_INDEX    (O_MOVE_ANGLE_INDEX + 0)
#define /*0x0C4*/ O_MOVE_ANGLE_YAW_INDEX      (O_MOVE_ANGLE_INDEX + 1)
#define /*0x0C4*/ O_MOVE_ANGLE_ROLL_INDEX     (O_MOVE_ANGLE_INDEX + 2)
#define /*0x0C4*/ oMoveAnglePitch             OBJECT_FIELD_S32(O_MOVE_ANGLE_PITCH_INDEX)
#define /*0x0C8*/ oMoveAngleYaw               OBJECT_FIELD_S32(O_MOVE_ANGLE_YAW_INDEX)
#define /*0x0CC*/ oMoveAngleRoll              OBJECT_FIELD_S32(O_MOVE_ANGLE_ROLL_INDEX)
#define /*0x0D0*/ O_FACE_ANGLE_INDEX          0x12
#define /*0x0D0*/ O_FACE_ANGLE_PITCH_INDEX    (O_FACE_ANGLE_INDEX + 0)
#define /*0x0D0*/ O_FACE_ANGLE_YAW_INDEX      (O_FACE_ANGLE_INDEX + 1)
#define /*0x0D0*/ O_FACE_ANGLE_ROLL_INDEX     (O_FACE_ANGLE_INDEX + 2)
#define /*0x0D0*/ oFaceAnglePitch             OBJECT_FIELD_S32(O_FACE_ANGLE_PITCH_INDEX)
#define /*0x0D4*/ oFaceAngleYaw               OBJECT_FIELD_S32(O_FACE_ANGLE_YAW_INDEX)
#define /*0x0D8*/ oFaceAngleRoll              OBJECT_FIELD_S32(O_FACE_ANGLE_ROLL_INDEX)
#define /*0x0DC*/ oGraphYOffset               OBJECT_FIELD_F32(0x15)
#define /*0x0E0*/ oActiveParticleFlags        OBJECT_FIELD_U32(0x16)
#define /*0x0E4*/ oGravity                    OBJECT_FIELD_F32(0x17)
#define /*0x0E8*/ oFloorHeight                OBJECT_FIELD_F32(0x18)
#define /*0x0EC*/ oMoveFlags                  OBJECT_FIELD_U32(0x19)
#define /*0x0F0*/ oAnimState                  OBJECT_FIELD_S32(0x1A)
// 0x0F4-0x110 (0x1B-0x22) are object specific and defined below the common fields.
#define /*0x114*/ oAngleVelPitch              OBJECT_FIELD_S32(0x23)
#define /*0x118*/ oAngleVelYaw                OBJECT_FIELD_S32(0x24)
#define /*0x11C*/ oAngleVelRoll               OBJECT_FIELD_S32(0x25)
#define /*0x120*/ oAnimations                 OBJECT_FIELD_ANIMS(0x26)
#define /*0x124*/ oHeldState                  OBJECT_FIELD_U32(0x27)
#define /*0x128*/ oWallHitboxRadius           OBJECT_FIELD_F32(0x28)
#define /*0x12C*/ oDragStrength               OBJECT_FIELD_F32(0x29)
#define /*0x130*/ oInteractType               OBJECT_FIELD_U32(0x2A)
#define /*0x134*/ oInteractStatus             OBJECT_FIELD_S32(0x2B)
#define /*0x138*/ O_PARENT_RELATIVE_POS_INDEX 0x2C
#define /*0x138*/ oParentRelativePosX         OBJECT_FIELD_F32(O_PARENT_RELATIVE_POS_INDEX + 0)
#define /*0x13C*/ oParentRelativePosY         OBJECT_FIELD_F32(O_PARENT_RELATIVE_POS_INDEX + 1)
#define /*0x140*/ oParentRelativePosZ         OBJECT_FIELD_F32(O_PARENT_RELATIVE_POS_INDEX + 2)
#define /*0x144*/ oBehParams2ndByte           OBJECT_FIELD_S32(0x2F)
// 0x148 unused, possibly a third param byte.
#define /*0x14C*/ oAction                     OBJECT_FIELD_S32(0x31)
#define /*0x150*/ oSubAction                  OBJECT_FIELD_S32(0x32)
#define /*0x154*/ oTimer                      OBJECT_FIELD_S32(0x33)
#define /*0x158*/ oBounce                     OBJECT_FIELD_F32(0x34)
#define /*0x15C*/ oDistanceToMario            OBJECT_FIELD_F32(0x35)
#define /*0x160*/ oAngleToMario               OBJECT_FIELD_S32(0x36)
#define /*0x164*/ oHomeX                      OBJECT_FIELD_F32(0x37)
#define /*0x168*/ oHomeY                      OBJECT_FIELD_F32(0x38)
#define /*0x16C*/ oHomeZ                      OBJECT_FIELD_F32(0x39)
#define /*0x170*/ oFriction                   OBJECT_FIELD_F32(0x3A)
#define /*0x174*/ oBuoyancy                   OBJECT_FIELD_F32(0x3B)
#define /*0x178*/ oSoundStateID               OBJECT_FIELD_S32(0x3C)
#define /*0x17C*/ oOpacity                    OBJECT_FIELD_S32(0x3D)
#define /*0x180*/ oDamageOrCoinValue          OBJECT_FIELD_S32(0x3E)
#define /*0x184*/ oHealth                     OBJECT_FIELD_S32(0x3F)
#define /*0x188*/ oBehParams                  OBJECT_FIELD_S32(0x40)
#define /*0x18C*/ oPrevAction                 OBJECT_FIELD_S32(0x41)
#define /*0x190*/ oInteractionSubtype         OBJECT_FIELD_U32(0x42)
#define /*0x194*/ oCollisionDistance          OBJECT_FIELD_F32(0x43)
#define /*0x198*/ oNumLootCoins               OBJECT_FIELD_S32(0x44)
#define /*0x19C*/ oDrawingDistance            OBJECT_FIELD_F32(0x45)
#define /*0x1A0*/ oRoom                       OBJECT_FIELD_S32(0x46)
// 0x1A4 is unused, possibly related to 0x1A8 in removed macro purposes.
#define /*0x1A8*/ oUnk1A8                     OBJECT_FIELD_U32(0x48)
// 0x1AC-0x1B2 (0x48-0x4A) are object specific and defined below the common fields.
#define /*0x1B4*/ oWallAngle                  OBJECT_FIELD_U32(0x4B)
#define /*0x1B8*/ oFloorType                  OBJECT_FIELD_S16(0x4C, 0)
#define /*0x1BA*/ oFloorRoom                  OBJECT_FIELD_S16(0x4C, 1)
#define /*0x1BC*/ oAngleToHome                OBJECT_FIELD_S32(0x4D)
#define /*0x1C0*/ oFloor                      OBJECT_FIELD_SURFACE(0x4E)
#define /*0x1C4*/ oDeathSound                 OBJECT_FIELD_S32(0x4F)

};

#endif
