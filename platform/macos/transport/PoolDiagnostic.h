#ifndef OPEN_DROBO_POOL_DIAGNOSTIC_H
#define OPEN_DROBO_POOL_DIAGNOSTIC_H

#include "CapacityWire.h"
#include <stdint.h>

typedef struct {
    uint64_t registry_id;
    uint64_t physical_registry_group;
    uint32_t mounted_volume_count;
    ODPoolDiagnosticWire wire;
} ODNativePoolDiagnostic;

/* Fixed read-only raw pool evidence, including multi-LUN devices. This does
 * not certify units, individual-volume accounting, or filesystem statistics.
 * The selected endpoint, physical registry group and mounted-volume count
 * must remain stable across capture. Every error clears the entire output.
 * No caller-supplied command or write selector is accepted. */
int32_t od_capture_pool_diagnostic(uint64_t registry_id, ODNativePoolDiagnostic *output);

#endif
