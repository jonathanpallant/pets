# Copyright (c) 2025 Ferrous Systems
# SPDX-License-Identifier: CC0-1.0

target extended-remote :1234
b HardFault
b PendSV
layout split
stepi
