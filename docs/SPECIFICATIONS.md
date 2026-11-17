# 📐 AngNative Technical Specifications

**Status:** Canonical Specifications (v1.0)  
**License:** MIT

---

## 1. Mutation Protocol Specification

The bridge between the Angular runtime and the Rust Host engine operates via a strictly typed, versioned mutation protocol.

### 1.1 Operation Types
Each mutation record contains an opcode and typed payload:

| Opcode | Command | Payload Fields | Description |
| :--- | :--- | :--- | :--- |
| `0x01` | `CreateNode` | `node_id: u64`, `tag: String` | Allocates a virtual node in the shadow tree |
| `0x02` | `SetProps` | `node_id: u64`, `props: Map<String, Value>` | Sets or updates component properties |
| `0x03` | `SetText` | `node_id: u64`, `content: String` | Sets or updates text node value |
| `0x04` | `InsertChild` | `parent_id: u64`, `child_id: u64`, `index: u32` | Inserts child into parent hierarchy |
| `0x05` | `RemoveChild` | `parent_id: u64`, `child_id: u64` | Detaches child node |
| `0x06` | `DropNode` | `node_id: u64` | Deallocates node and frees memory |

### 1.2 Serialization Format
- **Development / Testing**: JSON Command Batch array.
- **Production / FFI**: Compact binary buffer containing length-prefixed opcode tags and little-endian integers for maximum throughput.

---

## 2. Supported Styling Properties (Taffy Subset)

AngNative implements a high-performance subset of CSS Flexbox and Box Model properties evaluated by **Taffy 0.5.2**:

### 2.1 Layout & Flexbox
- `display`: `flex` | `none`
- `flex-direction`: `row` | `column` | `row-reverse` | `column-reverse`
- `justify-content`: `flex-start` | `center` | `flex-end` | `space-between` | `space-around` | `space-evenly`
- `align-items`: `flex-start` | `center` | `flex-end` | `stretch` | `baseline`
- `align-self`: `auto` | `flex-start` | `center` | `flex-end` | `stretch`
- `flex-grow`: `<number>` (e.g. `1`)
- `flex-shrink`: `<number>` (e.g. `0`)
- `flex-wrap`: `nowrap` | `wrap` | `wrap-reverse`
- `gap` / `row-gap` / `column-gap`: `<length>` (px, dp)

### 2.2 Box Model & Geometry
- `width`, `height`, `min-width`, `min-height`, `max-width`, `max-height`: `<length>` | `<percentage>` | `auto`
- `padding`, `padding-top`, `padding-right`, `padding-bottom`, `padding-left`: `<length>`
- `margin`, `margin-top`, `margin-right`, `margin-bottom`, `margin-left`: `<length>` | `auto`
- `border-width`, `border-radius`: `<length>`
- `position`: `relative` | `absolute`
- `top`, `right`, `bottom`, `left`: `<length>`

### 2.3 Visual Styles
- `background-color`: Hex (`#RRGGBB`, `#RRGGBBAA`), RGB, RGBA
- `color`: Font foreground color
- `font-size`: Numerical pixel/point values
- `font-weight`: `normal` | `bold` | `100`–`900`
- `opacity`: Float from `0.0` to `1.0`
- `visibility`: `visible` | `hidden`

---

## 3. C-FFI & JNI Interface Contract

### 3.1 C Export Header (`angnative_host_ffi.h`)
```c
#ifndef ANGNATIVE_HOST_FFI_H
#define ANGNATIVE_HOST_FFI_H

#include <stdint.h>
#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

// Opaque engine host handle
typedef struct AngNativeHostEngine AngNativeHostEngine;

// Lifecycle
AngNativeHostEngine* angnative_engine_create(void);
void angnative_engine_destroy(AngNativeHostEngine* engine);

// Mutation Dispatch
int32_t angnative_engine_dispatch_batch(
    AngNativeHostEngine* engine,
    const uint8_t* buffer,
    size_t length
);

// Layout Execution
void angnative_engine_compute_layout(
    AngNativeHostEngine* engine,
    float viewport_width,
    float viewport_height
);

#ifdef __cplusplus
}
#endif

#endif // ANGNATIVE_HOST_FFI_H
```

### 3.2 JNI Method Bindings (`JniBridge.kt`)
```kotlin
package com.angnative.app

object JniBridge {
    init {
        System.loadLibrary("angnative_host_ffi")
    }

    external fun initHostEngine(): Long
    external fun destroyHostEngine(handle: Long)
    external fun dispatchBatch(handle: Long, batchJson: String): Int
    external fun computeLayout(handle: Long, width: Float, height: Float)
}
```
