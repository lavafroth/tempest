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

#include <stddef.h>
#include <stdint.h>
#include <stdlib.h>
#include <stdio.h>
#include <assert.h>
#include <string.h>
#include "params.h"
#include "file/model_file.h"
#include "file/util.h"
#include "log.h"

#define MAX_NETWORKS 8

struct ModelFile_i {
    FILE *fd;

    size_t file_size;

    uint32_t version;
    size_t header_offset;
    size_t header_size;

    // https://en.wikipedia.org/wiki/IETF_language_tag
    char language[9];

    char *name;
    char *description;
    // could add copyright, author, website strings

    ModelType type;
    size_t params_offset;
    size_t params_size;

    size_t num_networks;
    struct {
        size_t offset;
        size_t size;
    } networks[MAX_NETWORKS];
};

const char *MODEL_EXPECTED_MAGIC = "APRILMDL";
bool read_metadata(ModelFile model) {
    FILE *fd = model->fd;

    fseek(fd, 0L, SEEK_END);
    model->file_size = ftell(fd);

    fseek(fd, 0L, SEEK_SET);
    char magic[8];
    fread(magic, 1, 8, fd);

    if(memcmp(magic, MODEL_EXPECTED_MAGIC, 8) != 0) {
        LOG_INFO("Magic check failed");
        return false;
    }

    uint32_t version = mfu_read_u32(fd);
    model->version = version;
    if(version != 1) {
        LOG_WARNING("Unsupported model version %u", version);
        return false;
    }

    uint64_t header_size = mfu_read_u64(fd);
    model->header_size = header_size;

    model->header_offset = ftell(fd);

    return true;
}

bool read_header(ModelFile model) {
    if(model->header_offset < 8) return false;

    FILE *fd = model->fd;
    fseek(fd, model->header_offset, SEEK_SET);

    fread(model->language, 1, 8, fd);
    model->language[8] = '\0';
    
    model->name = mfu_alloc_read_string(fd);
    model->description = mfu_alloc_read_string(fd);

    model->type = (ModelType)mfu_read_u32(fd); // TODO: check network count equal LSTM_TRANSDUCER_STATELESS_NETWORK_COUNT
    if(!((model->type > MODEL_UNKNOWN) && (model->type < MODEL_MAX))) {
        LOG_WARNING("Unexpected model type %u", model->type);
        return false;
    }

    model->params_offset = mfu_read_u64(fd);
    model->params_size = mfu_read_u64(fd);
    if((model->params_offset + model->params_size) > model->file_size) {
        LOG_WARNING("Params out of bounds of file");
        return false;
    }

    model->num_networks = mfu_read_u64(fd);
    if(model->num_networks > MAX_NETWORKS) {
        LOG_WARNING("Too many networks %zu", model->num_networks);
        return false;
    }

    for(size_t i=0; i<model->num_networks; i++){
        model->networks[i].offset = mfu_read_u64(fd);
        model->networks[i].size = mfu_read_u64(fd);
        if((model->networks[i].offset + model->networks[i].size) > model->file_size) {
            LOG_WARNING("Network %zu out of bounds of file", i);
            return false;
        }
    }

    return true;
}

ModelFile model_read(const char *path) {
    FILE *fd = fopen(path, "rb");
    if(!fd) return NULL;

    ModelFile model = (ModelFile)calloc(1, sizeof(struct ModelFile_i));
    model->fd = fd;

    if(!read_metadata(model)){
        free_model(model);
        return NULL;
    }

    if(!read_header(model)){
        free_model(model);
        return NULL;
    }

    return model;
}


const char *model_name(ModelFile model) {
    return model->name;
}

const char *model_desc(ModelFile model) {
    return model->description;
}

ModelType model_type(ModelFile model) {
    return model->type;
}

bool model_read_params(ModelFile model, ModelParameters *out) {
    return read_params_from_fd(out, model->fd);
}

size_t model_network_count(ModelFile model) {
    return model->num_networks;
}

size_t model_network_size(ModelFile model, size_t index) {
    return model->networks[index].size;
}

size_t model_network_read(ModelFile model, size_t index, void *data, size_t data_len) {
    // TODO: Should mmap instead!!!
    if(data_len > model_network_size(model, index))
        data_len = model_network_size(model, index);
    
    FILE *fd = model->fd;
    fseek(fd, model->networks[index].offset, SEEK_SET);

    return fread(data, 1, data_len, fd);
}

void transfer_strings_and_free_model(ModelFile model, char **out_name, char **out_desc, char **out_lang) {
    fclose(model->fd);

    if(out_name != NULL){
        *out_name = model->name;
    }else{
        free(model->name);
    }

    if(out_desc != NULL){
        *out_desc = model->description;
    }else{
        free(model->description);
    }

    if(out_lang != NULL){
        *out_lang = (char *)malloc(strlen(model->language) + 1);
        strcpy(*out_lang, model->language);
    }

    free(model);
}

void free_model(ModelFile model) {
    transfer_strings_and_free_model(model, NULL, NULL, NULL);
}