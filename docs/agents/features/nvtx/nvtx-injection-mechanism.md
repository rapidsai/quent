# NVTX Injection Mechanism

## Overview

NVTX v3 is a **header-only** library. All NVTX API functions
(`nvtxRangePushA`, `nvtxDomainMarkEx`, etc.) are compiled directly into the
application binary. Each function dispatches through a function pointer stored
in a global table. By default, these pointers are null — all calls are no-ops.

An **injection library** is a shared library that replaces these function
pointers with real implementations. There is no separate `libnvtx.so` to link
against. The injection library is the only runtime component.

## Loading

The injection library is loaded on the first call to any NVTX API function.
Initialization is one-shot and thread-safe (atomic CAS on an init flag).

### Search order

1. **Environment variable**: `NVTX_INJECTION64_PATH` (64-bit) or
   `NVTX_INJECTION32_PATH` (32-bit). Value is the absolute path to a
   `.so` / `.dll`.
2. **Android only**: `libNvtxInjection64.so` in
   `/data/data/<package>/files/`.
3. **Static injection** (GCC/ELF only): A weak symbol
   `InitializeInjectionNvtx2_fnptr`. If a static library defines this as a
   normal (non-weak) symbol, the linker resolves it and NVTX calls through it.

### Platform specifics

- Linux: `dlopen(path, RTLD_LAZY)` then
  `dlsym(handle, "InitializeInjectionNvtx2")`
- Windows: `LoadLibraryW(path)` then
  `GetProcAddress(handle, "InitializeInjectionNvtx2")`

## Required exports

An injection library must export with `extern "C"` linkage:

```c
// Called once on first NVTX API usage. Return 1 for success, 0 for failure.
int InitializeInjectionNvtx2(NvtxGetExportTableFunc_t getExportTable);
```

On failure, all NVTX functions remain no-ops.

## Registration flow

The `getExportTable` parameter retrieves export tables by ID:

```c
typedef const void* (NVTX_API * NvtxGetExportTableFunc_t)(uint32_t exportTableId);
```

### Step 1: Get the callbacks export table

```c
const NvtxExportTableCallbacks* callbacks =
    (const NvtxExportTableCallbacks*)getExportTable(NVTX_ETID_CALLBACKS);
```

### Step 2: Get a module's function table

```c
NvtxFunctionTable table = nullptr;
unsigned int tableSize = 0;
callbacks->GetModuleFunctionTable(NVTX_CB_MODULE_CORE, &table, &tableSize);
```

`NvtxFunctionTable` is `NvtxFunctionPointer**` — an array of pointers to
function pointers.

### Step 3: Assign callback implementations

```c
*table[NVTX_CBID_CORE_RangePushA] = (NvtxFunctionPointer)MyRangePushA;
*table[NVTX_CBID_CORE_RangePop]   = (NvtxFunctionPointer)MyRangePop;
```

Note the double-dereference: `table[idx]` is a `NvtxFunctionPointer*`, so
the write goes through `*table[idx]`.

## Callback modules

### NVTX_CB_MODULE_CORE (1) — original (non-domain) API

| ID | Name | Signature |
|----|------|-----------|
| 1  | MarkEx | `void(const nvtxEventAttributes_t*)` |
| 2  | MarkA | `void(const char*)` |
| 3  | MarkW | `void(const wchar_t*)` |
| 4  | RangeStartEx | `nvtxRangeId_t(const nvtxEventAttributes_t*)` |
| 5  | RangeStartA | `nvtxRangeId_t(const char*)` |
| 6  | RangeStartW | `nvtxRangeId_t(const wchar_t*)` |
| 7  | RangeEnd | `void(nvtxRangeId_t)` |
| 8  | RangePushEx | `int(const nvtxEventAttributes_t*)` |
| 9  | RangePushA | `int(const char*)` |
| 10 | RangePushW | `int(const wchar_t*)` |
| 11 | RangePop | `int(void)` |
| 12 | NameCategoryA | `void(uint32_t, const char*)` |
| 13 | NameCategoryW | `void(uint32_t, const wchar_t*)` |
| 14 | NameOsThreadA | `void(uint32_t, const char*)` |
| 15 | NameOsThreadW | `void(uint32_t, const wchar_t*)` |

### NVTX_CB_MODULE_CORE2 (5) — domain-aware API

| ID | Name | Signature |
|----|------|-----------|
| 1  | DomainMarkEx | `void(nvtxDomainHandle_t, const nvtxEventAttributes_t*)` |
| 2  | DomainRangeStartEx | `nvtxRangeId_t(nvtxDomainHandle_t, const nvtxEventAttributes_t*)` |
| 3  | DomainRangeEnd | `void(nvtxDomainHandle_t, nvtxRangeId_t)` |
| 4  | DomainRangePushEx | `int(nvtxDomainHandle_t, const nvtxEventAttributes_t*)` |
| 5  | DomainRangePop | `int(nvtxDomainHandle_t)` |
| 6  | DomainResourceCreate | `nvtxResourceHandle_t(nvtxDomainHandle_t, nvtxResourceAttributes_t*)` |
| 7  | DomainResourceDestroy | `void(nvtxResourceHandle_t)` |
| 8  | DomainNameCategoryA | `void(nvtxDomainHandle_t, uint32_t, const char*)` |
| 9  | DomainNameCategoryW | `void(nvtxDomainHandle_t, uint32_t, const wchar_t*)` |
| 10 | DomainRegisterStringA | `nvtxStringHandle_t(nvtxDomainHandle_t, const char*)` |
| 11 | DomainRegisterStringW | `nvtxStringHandle_t(nvtxDomainHandle_t, const wchar_t*)` |
| 12 | DomainCreateA | `nvtxDomainHandle_t(const char*)` |
| 13 | DomainCreateW | `nvtxDomainHandle_t(const wchar_t*)` |
| 14 | DomainDestroy | `void(nvtxDomainHandle_t)` |
| 15 | Initialize | `void(const void*)` |

## What callbacks do NOT provide

- **Timestamps**: The injection must capture its own timestamps.
- **Thread ID**: Not passed as a parameter. Must use OS APIs
  (`gettid()`, `pthread_self()`, `GetCurrentThreadId()`).
- **Stack depth for push/pop**: The injection returns a depth value from
  `RangePush*`. If depth tracking is needed, the injection must maintain
  its own per-thread counter.

## Opaque handle types

The injection library **defines** the struct content for opaque handle types.
NVTX only forward-declares them:

- `nvtxDomainHandle_t` — pointer to injection-defined domain struct
- `nvtxStringHandle_t` — pointer to injection-defined string struct
- `nvtxResourceHandle_t` — pointer to injection-defined resource struct

## Key data structures

### nvtxEventAttributes_t

```c
typedef struct nvtxEventAttributes_v2 {
    uint16_t version;       // NVTX_VERSION (3)
    uint16_t size;          // sizeof(nvtxEventAttributes_t)
    uint32_t category;      // User-defined category ID
    int32_t  colorType;     // NVTX_COLOR_ARGB or NVTX_COLOR_UNKNOWN
    uint32_t color;         // ARGB color value
    int32_t  payloadType;   // nvtxPayloadType_t
    int32_t  reserved0;
    union {
        uint64_t ullValue;
        int64_t  llValue;
        double   dValue;
        uint32_t uiValue;
        int32_t  iValue;
        float    fValue;
    } payload;
    int32_t  messageType;   // nvtxMessageType_t
    nvtxMessageValue_t message;  // ascii, unicode, or registered handle
} nvtxEventAttributes_v2;
```

## Source references

- NVTX v3 repository: https://github.com/NVIDIA/NVTX
- Headers: `c/include/nvtx3/`
- Sample injection: `tools/sample-injection/Source/NvtxSampleInjection.cpp`
