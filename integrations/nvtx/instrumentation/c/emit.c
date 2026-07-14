/* SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved. */
/* SPDX-License-Identifier: Apache-2.0 */

/*
 * Deterministic NVTX v3 emitter primitives for the quent-nvtx capture
 * end-to-end test.
 *
 * This is the "instrumented application" role: it makes ordinary NVTX v3 client
 * calls and knows nothing about Quent. Capture happens only because the harness
 * sets NVTX_INJECTION64_PATH to the quent-nvtx capture cdylib before the first
 * NVTX call (Pitfall 1).
 *
 * Each function is a thin wrapper over one NVTX v3 client entry point so the
 * Rust `nvtx_test_app` can orchestrate a fixed, multi-threaded timeline that
 * exercises EVERY core NVTX kind (D-11). Handles (domain, resource) are returned
 * as opaque `void*`; range ids and registered-string handles as `uint64_t` —
 * all so the Rust side can hand them across threads and back into these calls.
 */

#include <nvtx3/nvToolsExt.h>

#include <stdint.h>
#include <sys/syscall.h>
#include <unistd.h>

static void fill_event(nvtxEventAttributes_t* attr, const char* label) {
    /* Zero-initialize, then set version/size so the injection's size-bounded
     * reads see a well-formed struct (Pitfall 4). */
    for (unsigned i = 0; i < sizeof(*attr); ++i) {
        ((char*)attr)[i] = 0;
    }
    attr->version = NVTX_VERSION;
    attr->size = NVTX_EVENT_ATTRIB_STRUCT_SIZE;
    attr->messageType = NVTX_MESSAGE_TYPE_ASCII;
    attr->message.ascii = label;
}

/* --- domains -------------------------------------------------------------- */

void* quent_nvtx_emit_domain_create(const char* name) {
    return (void*)nvtxDomainCreateA(name);
}

void quent_nvtx_emit_domain_destroy(void* domain) {
    nvtxDomainDestroy((nvtxDomainHandle_t)domain);
}

/* --- registered strings / categories / thread naming ---------------------- */

uint64_t quent_nvtx_emit_register_string(void* domain, const char* s) {
    return (uint64_t)(uintptr_t)nvtxDomainRegisterStringA((nvtxDomainHandle_t)domain, s);
}

void quent_nvtx_emit_name_category(void* domain, uint32_t category, const char* name) {
    nvtxDomainNameCategoryA((nvtxDomainHandle_t)domain, category, name);
}

/* Name the CALLING OS thread and return its tid, so the Rust orchestrator can
 * assert a distinct NameThread per spawned thread (D-11). */
uint32_t quent_nvtx_emit_name_current_thread(const char* name) {
    uint32_t tid = (uint32_t)syscall(SYS_gettid);
    nvtxNameOsThreadA(tid, name);
    return tid;
}

/* --- marks ---------------------------------------------------------------- */

/* Emit a domain mark carrying a CORE payload union (D-12): an unsigned-int64. */
void quent_nvtx_emit_mark(void* domain, const char* msg, uint64_t payload) {
    nvtxEventAttributes_t attr;
    fill_event(&attr, msg);
    attr.payloadType = NVTX_PAYLOAD_TYPE_UNSIGNED_INT64;
    attr.payload.ullValue = payload;
    nvtxDomainMarkEx((nvtxDomainHandle_t)domain, &attr);
}

/* --- push/pop (per-thread nested ranges) ---------------------------------- */

void quent_nvtx_emit_push(void* domain, const char* label) {
    nvtxEventAttributes_t attr;
    fill_event(&attr, label);
    nvtxDomainRangePushEx((nvtxDomainHandle_t)domain, &attr);
}

void quent_nvtx_emit_pop(void* domain) {
    nvtxDomainRangePop((nvtxDomainHandle_t)domain);
}

/* --- start/end (process-wide ranges, may cross threads) ------------------- */

uint64_t quent_nvtx_emit_range_start(void* domain, const char* label) {
    nvtxEventAttributes_t attr;
    fill_event(&attr, label);
    return (uint64_t)nvtxDomainRangeStartEx((nvtxDomainHandle_t)domain, &attr);
}

void quent_nvtx_emit_range_end(void* domain, uint64_t id) {
    nvtxDomainRangeEnd((nvtxDomainHandle_t)domain, (nvtxRangeId_t)id);
}

/* --- resources ------------------------------------------------------------ */

void* quent_nvtx_emit_resource_create(void* domain, int32_t id_type, uint64_t identifier,
                                      const char* name) {
    nvtxResourceAttributes_t attr;
    for (unsigned i = 0; i < sizeof(attr); ++i) {
        ((char*)&attr)[i] = 0;
    }
    attr.version = NVTX_VERSION;
    attr.size = NVTX_RESOURCE_ATTRIB_STRUCT_SIZE;
    attr.identifierType = id_type;
    attr.identifier.ullValue = identifier;
    attr.messageType = NVTX_MESSAGE_TYPE_ASCII;
    attr.message.ascii = name;
    return (void*)nvtxDomainResourceCreate((nvtxDomainHandle_t)domain, &attr);
}

void quent_nvtx_emit_resource_destroy(void* resource) {
    nvtxDomainResourceDestroy((nvtxResourceHandle_t)resource);
}
