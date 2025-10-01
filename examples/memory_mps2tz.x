/* Memory Configuration Linker Script

This file is imported by cortex-m-rt's link.x script, and should be placed
somewhere in the linker's search path.

Copyright (c) 2025 Ferrous Systems
SPDX-License-Identifier: CC0-1.0
*/

/*
Settings for AN505 on MPS2
*/

MEMORY
{
  FLASH : ORIGIN = 0x10000000, LENGTH = 4M
    RAM : ORIGIN = 0x28000000, LENGTH = 4M
  PSRAM : ORIGIN = 0x80000000, LENGTH = 16M
}
