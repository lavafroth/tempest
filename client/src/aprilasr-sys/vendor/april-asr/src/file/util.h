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

#ifndef _APRIL_MODEL_FILE_UTIL
#define _APRIL_MODEL_FILE_UTIL

#ifdef _MSC_VER

// Assuming Windows is always little-endian
#define le32toh(x) x
#define le64toh(x) x

#elif __APPLE__

// Assuming OSX is always little-endian
#define le32toh(x) x
#define le64toh(x) x

#else
#include <endian.h>
#endif

#include <stdint.h>
#include <stdlib.h>
#include <stdio.h>
#include "log.h"

static inline uint32_t mfu_read_u32(FILE *fd) {
    uint32_t v;
    fread(&v, sizeof(uint32_t), 1, fd);
    v = le32toh(v);
    return v;
}

static inline uint64_t mfu_read_u64(FILE *fd) {
    uint64_t v;
    fread(&v, sizeof(uint64_t), 1, fd);
    v = le64toh(v);
    return v;
}

static inline int32_t mfu_read_i32(FILE *fd) {
    uint32_t v;
    fread(&v, sizeof(uint32_t), 1, fd);
    v = le32toh(v);
    return *((int32_t *)&v);
}

static inline int64_t mfu_read_i64(FILE *fd) {
    uint64_t v;
    fread(&v, sizeof(uint64_t), 1, fd);
    v = le64toh(v);
    return *((int64_t *)&v);
}

// Must be freed manually with free(v)
static inline char *mfu_alloc_read_string(FILE *fd) {
    uint64_t size = mfu_read_u64(fd);
    char *v = (char *)malloc(size + 1);
    if(v == NULL) {
        LOG_ERROR("failed allocating string of size %zu, file position %ld", size, ftell(fd));
        exit(-1);
    }
    fread(v, 1, size, fd);
    v[size] = '\0';
    return v;
}

#endif