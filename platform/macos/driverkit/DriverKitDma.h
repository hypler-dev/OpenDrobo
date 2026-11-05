#ifndef OPENDROBO_DRIVERKIT_DMA_H
#define OPENDROBO_DRIVERKIT_DMA_H

#include "DmaOwnership.h"

#ifdef __cplusplus
class IOPCIDevice;

/*
 * The controller supplies its already-open DMA-owning PCI provider. The factory
 * reference is borrowed only until a lease creates its command; each successful
 * command retains the provider through release_command. Session close/release
 * is not evidence that bus mastering has drained. This adapter opens no hardware.
 */
ODDmaBackend od_driverkit_dma_backend(IOPCIDevice *provider);

/*
 * Copies caller-owned source bytes into an already prepared DriverKit DMA
 * mapping. bytes must remain valid for length during this synchronous call.
 * The lease remains prepared and owned by its caller on every result.
 */
int od_driverkit_dma_write_request(ODDmaLease *lease, const void *bytes,
                                   uint32_t length);
#endif

#endif
