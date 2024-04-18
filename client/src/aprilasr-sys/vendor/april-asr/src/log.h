/*
 * Copyright (C) 2022 abb128
 * 
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, version 3.
 * 
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 * 
 * You should have received a copy of the GNU General Public License
 * along with this program.  If not, see <https://www.gnu.org/licenses/>.
*/

#ifndef _APRIL_LOG
#define _APRIL_LOG

#include <stdio.h>
#include "common.h"

typedef enum LogLevel {
    LEVEL_DEBUG = 0,
    LEVEL_INFO,
    LEVEL_WARNING,
    LEVEL_ERROR,
    LEVEL_COUNT
} LogLevel;

static const char LogLevelStrings[5][10] = {
    "DEBUG",
    "INFO",
    "WARNING",
    "ERROR",
    "NONE"
};

static const char LogLevelColors[4][16] = {
    "\033[0m",
    "\033[36;1m",
    "\033[33;1m",
    "\033[31;1m\a"
};

extern LogLevel g_loglevel;

#define S1(x) #x
#define S2(x) S1(x)
#define LOCATION __FILE__ ":" S2(__LINE__)
#define LOG_WITH_LEVEL(level, fmt, ...) if(level >= g_loglevel) fprintf(stderr, "libapril: " "(" LOCATION ")" " %s[%s]\033[0m " fmt "\n", LogLevelColors[level], LogLevelStrings[level], ##__VA_ARGS__)

#define LOG_DEBUG(fmt, ...)    LOG_WITH_LEVEL(LEVEL_DEBUG,    fmt, ##__VA_ARGS__)
#define LOG_INFO(fmt, ...)     LOG_WITH_LEVEL(LEVEL_INFO,     fmt, ##__VA_ARGS__)
#define LOG_WARNING(fmt, ...)  LOG_WITH_LEVEL(LEVEL_WARNING,  fmt, ##__VA_ARGS__)
#define LOG_ERROR(fmt, ...)    LOG_WITH_LEVEL(LEVEL_ERROR,    fmt, ##__VA_ARGS__)


#endif