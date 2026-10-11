#!/usr/bin/env python3
"""Builds the synthetic SCM image for the scm_vm_core parity case.

The image is a minimal but layout-faithful GTA III main.scm:
  - offset 0:      GOTO chain header (operand doubles as variable-space size)
  - offset 8..VS:  4 global variable slots (i32 each)
  - VS:            second GOTO (operand = object-table end, doubles as object size)
  - VS+8:          used-object table (i16 count = 0)
  - OBJ_END:       third GOTO to main code
  - OBJ_END+8:     multiscript header (main size, largest mission, i16 count = 0)
  - MAIN_CODE:     the test program

re3 references for the layout:
  - references/re-GTA/src/control/Script.cpp  (CTheScripts::Init)
  - references/re-GTA/src/control/Script5.cpp (ReadObjectNamesFromScript,
    ReadMultiScriptFileOffsetsFromScript)

Run from this directory:  python build_image.py
Writes image.hex next to itself.
"""

from __future__ import annotations

import struct
from pathlib import Path

# Opcode ids extracted from references/re-GTA/src/control/ScriptCommands.h (enum order).
NOP = 0
WAIT = 1
GOTO = 2
SET_VAR_INT = 4
SET_VAR_FLOAT = 5
SET_LVAR_INT = 6
SET_LVAR_FLOAT = 7
ADD_VAL_TO_INT_VAR = 8
ADD_VAL_TO_INT_LVAR = 10
ADD_VAL_TO_FLOAT_LVAR = 11
SUB_VAL_FROM_INT_LVAR = 14
MULT_INT_VAR_BY_VAL = 16
MULT_INT_LVAR_BY_VAL = 18
DIV_INT_LVAR_BY_VAL = 22
IS_INT_VAR_GREATER_THAN_NUMBER = 24
IS_INT_LVAR_GREATER_THAN_NUMBER = 25
IS_NUMBER_GREATER_THAN_INT_VAR = 26
GOTO_IF_FALSE = 77
TERMINATE_THIS_SCRIPT = 78
START_NEW_SCRIPT = 79
GOSUB = 80
RETURN = 81
RETURN_TRUE = 197
ANDOR = 214
LAUNCH_MISSION = 215
MISSION_HAS_FINISHED = 216

# eScriptArgument stream type bytes (references/re-GTA/src/control/Script.h).
ARG_END = 0
ARG_INT32 = 1
ARG_GLOBALVAR = 2
ARG_LOCALVAR = 3
ARG_INT8 = 4
ARG_INT16 = 5
ARG_FLOAT = 6

G8 = 8
G12 = 12
G16 = 16
G20 = 20


class Assembler:
    def __init__(self) -> None:
        self.bytes = bytearray()
        self.labels: dict[str, int] = {}
        self.fixups: list[tuple[int, str]] = []

    @property
    def pc(self) -> int:
        return len(self.bytes)

    def label(self, name: str) -> None:
        self.labels[name] = self.pc

    def op(self, opcode: int, not_flag: bool = False) -> None:
        raw = opcode | (0x8000 if not_flag else 0)
        self.bytes += struct.pack("<H", raw)

    def int32(self, value: int) -> None:
        self.bytes.append(ARG_INT32)
        self.bytes += struct.pack("<i", value)

    def int8(self, value: int) -> None:
        self.bytes.append(ARG_INT8)
        self.bytes += struct.pack("<b", value)

    def int16(self, value: int) -> None:
        self.bytes.append(ARG_INT16)
        self.bytes += struct.pack("<h", value)

    def float16(self, value: float) -> None:
        self.bytes.append(ARG_FLOAT)
        self.bytes += struct.pack("<h", int(value * 16))

    def globalvar(self, offset: int) -> None:
        self.bytes.append(ARG_GLOBALVAR)
        self.bytes += struct.pack("<H", offset)

    def localvar(self, index: int) -> None:
        self.bytes.append(ARG_LOCALVAR)
        self.bytes += struct.pack("<H", index)

    def label_ref(self, name: str) -> None:
        self.bytes.append(ARG_INT32)
        self.fixups.append((self.pc, name))
        self.bytes += struct.pack("<i", 0)

    def goto(self, target: str) -> None:
        self.op(GOTO)
        self.label_ref(target)

    def build(self) -> bytearray:
        for at, name in self.fixups:
            struct.pack_into("<i", self.bytes, at, self.labels[name])
        return self.bytes


def assemble() -> bytearray:
    a = Assembler()

    # ---- header segment 1: GOTO to variable space end (= main code chain) ----
    a.op(GOTO)
    a.int32(24)  # variable space size (4 global slots at 8..24)
    a.bytes.append(0)  # pad to offset 8
    assert a.pc == 8
    for _ in range(4):
        a.bytes += struct.pack("<i", 0)  # global slots G8, G12, G16, G20
    assert a.pc == 24

    # ---- header segment 2: used objects table ----
    a.op(GOTO)
    a.int32(36)  # object size: offset of the mission header segment
    a.bytes.append(ARG_INT16)  # cosmetic type byte before the count
    a.bytes += struct.pack("<h", 0)  # NumberOfUsedObjects = 0
    a.bytes += struct.pack("<h", 0)  # pad
    assert a.pc == 36

    # ---- header segment 3: multiscript header ----
    a.op(GOTO)
    a.label_ref("main")
    a.bytes.append(0)  # pad to segment + 8
    main_script_size_pos = a.pc
    a.bytes += struct.pack("<i", 0)  # MainScriptSize (patched at the end)
    a.bytes += struct.pack("<i", 0)  # LargestMissionScriptSize
    a.bytes += struct.pack("<h", 0)  # NumberOfMissionScripts = 0
    a.bytes += struct.pack("<h", 0)  # pad
    assert a.pc == 56

    # ---- main thread ----
    a.label("main")
    a.op(SET_LVAR_INT); a.localvar(0); a.int8(5)
    a.op(SET_VAR_INT); a.globalvar(G8); a.int32(100)
    a.op(ADD_VAL_TO_INT_VAR); a.globalvar(G8); a.int8(23)
    a.op(SUB_VAL_FROM_INT_LVAR); a.localvar(0); a.int8(2)
    a.op(MULT_INT_VAR_BY_VAL); a.globalvar(G8); a.int8(2)
    a.op(DIV_INT_LVAR_BY_VAL); a.localvar(0); a.int8(2)
    a.op(SET_LVAR_FLOAT); a.localvar(4); a.float16(2.5)
    a.op(ADD_VAL_TO_FLOAT_LVAR); a.localvar(4); a.float16(0.5)
    a.op(IS_INT_LVAR_GREATER_THAN_NUMBER); a.localvar(0); a.int8(0)
    a.op(GOTO_IF_FALSE); a.label_ref("skip1")
    a.op(SET_LVAR_INT); a.localvar(1); a.int8(111)
    a.label("skip1")
    a.op(ANDOR); a.int8(2)  # ANDS_2: this and the next two conditions
    a.op(IS_INT_VAR_GREATER_THAN_NUMBER); a.globalvar(G8); a.int8(100)
    a.op(IS_INT_LVAR_GREATER_THAN_NUMBER); a.localvar(0); a.int8(5)
    a.op(IS_INT_LVAR_GREATER_THAN_NUMBER); a.localvar(1); a.int8(100)
    a.op(GOTO_IF_FALSE); a.label_ref("else1")
    a.op(SET_LVAR_INT); a.localvar(2); a.int8(99)
    a.label("else1")
    a.op(ANDOR); a.int8(22)  # ORS_2
    a.op(IS_INT_LVAR_GREATER_THAN_NUMBER); a.localvar(0); a.int8(5)
    a.op(IS_INT_LVAR_GREATER_THAN_NUMBER, not_flag=True); a.localvar(1); a.int8(100)
    a.op(IS_INT_LVAR_GREATER_THAN_NUMBER); a.localvar(1); a.int8(0)
    a.op(GOTO_IF_FALSE); a.label_ref("else2")
    a.op(SET_LVAR_INT); a.localvar(3); a.int8(42)
    a.label("else2")
    a.op(GOSUB); a.label_ref("sub_a")
    a.op(GOSUB); a.label_ref("sub_b")
    a.op(WAIT); a.int32(32)
    a.op(START_NEW_SCRIPT); a.label_ref("thread_b"); a.int8(9); a.localvar(0); a.bytes.append(ARG_END)
    a.op(SET_VAR_FLOAT); a.globalvar(G12); a.float16(1.25)
    a.op(IS_NUMBER_GREATER_THAN_INT_VAR); a.int16(200); a.globalvar(G8)
    a.op(GOTO_IF_FALSE); a.label_ref("end_main")
    a.op(NOP)
    a.label("end_main")
    a.op(WAIT); a.int8(0)
    a.goto("second_phase")
    a.label("second_phase")
    # Mission thread terminates at t=64; the freed slot must be the next one
    # StartNewScript hands out (re3 LIFO: terminated slots return to the idle
    # list head). WAIT 80 at t=48 wakes at t=128, after the termination.
    a.op(WAIT); a.int32(80)
    a.op(START_NEW_SCRIPT); a.label_ref("thread_c"); a.bytes.append(ARG_END)
    a.label("main_idle")
    a.op(WAIT); a.int8(0)
    a.goto("main_idle")

    a.label("sub_a")
    a.op(SET_LVAR_INT); a.localvar(5); a.int8(7)
    a.op(GOSUB); a.label_ref("sub_c")
    a.op(IS_INT_LVAR_GREATER_THAN_NUMBER); a.localvar(5); a.int8(6)
    a.op(RETURN_TRUE)
    a.op(RETURN)

    a.label("sub_b")
    a.op(SET_LVAR_INT); a.localvar(6); a.int8(8)
    a.op(RETURN)

    a.label("sub_c")
    a.op(MULT_INT_LVAR_BY_VAL); a.localvar(5); a.int8(3)
    a.op(RETURN)

    a.label("thread_b")
    a.op(SET_VAR_INT); a.globalvar(G16); a.int16(1000)
    a.op(LAUNCH_MISSION); a.label_ref("mission_t")
    a.label("thread_b_loop")
    a.op(WAIT); a.int8(0)
    a.goto("thread_b_loop")

    a.label("thread_c")
    a.op(SET_VAR_INT); a.globalvar(G20); a.int8(7)
    a.label("thread_c_loop")
    a.op(WAIT); a.int8(0)
    a.goto("thread_c_loop")

    a.label("mission_t")
    a.op(SET_LVAR_INT); a.localvar(0); a.int8(77)
    a.op(GOSUB); a.label_ref("sub_d")
    a.op(MISSION_HAS_FINISHED)
    a.op(TERMINATE_THIS_SCRIPT)

    a.label("sub_d")
    a.op(ADD_VAL_TO_INT_LVAR); a.localvar(0); a.int8(1)
    a.op(RETURN)

    image = a.build()
    struct.pack_into("<i", image, main_script_size_pos, len(image))
    return image


def main() -> None:
    image = assemble()
    out = Path(__file__).resolve().parent / "image.hex"
    hex_lines = []
    for row in range(0, len(image), 32):
        hex_lines.append(" ".join(f"{byte:02X}" for byte in image[row : row + 32]))
    out.write_text("\n".join(hex_lines) + "\n", encoding="ascii")
    print(f"wrote {out} ({len(image)} bytes)")


if __name__ == "__main__":
    main()
