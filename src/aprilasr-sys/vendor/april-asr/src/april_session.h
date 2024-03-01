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

#ifndef _APRIL_SESSION
#define _APRIL_SESSION

#include "common.h"
#include <stdbool.h>
#include "ort_util.h"
#include "april_model.h"
#include "april_api.h"
#include "fbank.h"

#include "audio_provider.h"
#include "proc_thread.h"

#define MAX_ACTIVE_TOKENS 72

struct AprilASRSession_i {
    AprilASRModel model;
    OnlineFBank fbank;

    OrtMemoryInfo *memory_info;

    TensorF x;

    bool hc_use_0;
    TensorF h[2];
    TensorF c[2];

    TensorF eout;

    size_t context_size;
    TensorI context;
    TensorF dout;
    bool dout_init;

    TensorF logits;

    AprilToken active_tokens[MAX_ACTIVE_TOKENS];
    size_t active_token_head;
    size_t last_handler_call_head;

    bool emitted_silence;
    bool was_flushed;

    bool sync;
    bool force_realtime;
    AudioProvider provider;
    ProcThread thread;

    size_t current_time_ms;
    size_t last_emission_time_ms;

    AprilRecognitionResultHandler handler;
    void *userdata;

    size_t time_since_update_speed;
    double speed_needed;
};

#endif