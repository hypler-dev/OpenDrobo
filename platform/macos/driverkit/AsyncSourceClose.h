#ifndef OPENDROBO_ASYNC_SOURCE_CLOSE_H
#define OPENDROBO_ASYNC_SOURCE_CLOSE_H
#include <stdint.h>

enum class ODAsyncClosePhase { Idle, Disabling, Cancelling, Failed, Closed };
struct ODAsyncCloseState {
    ODAsyncClosePhase phase = ODAsyncClosePhase::Idle;
    int32_t error = 0;
};

/* Queue-only cancellation shares the same retained-failure/retry contract. */
template <typename Source>
int32_t od_async_source_cancel(Source *source, ODAsyncCloseState *state,
                               void (^completed)(), void (^failed)(int32_t)) {
    if (!source || !state || !completed || !failed) return -1;
    if (state->phase == ODAsyncClosePhase::Closed || state->phase == ODAsyncClosePhase::Cancelling) return 0;
    state->phase = ODAsyncClosePhase::Cancelling;
    state->error = 0;
    auto cancelled = source->Cancel(^{
        state->phase = ODAsyncClosePhase::Closed;
        state->error = 0;
        completed();
    });
    if (cancelled != 0) {
        state->phase = ODAsyncClosePhase::Failed;
        state->error = cancelled;
        failed(cancelled);
    }
    return cancelled;
}

/* The owner/source/queue must remain retained until completion. Failure leaves
 * ownership unchanged and permits an explicit retry; it never fabricates drain.
 * The SDK and synthetic sources share these exact callback transitions. */
template <typename Source>
int32_t od_async_source_close(Source *source, ODAsyncCloseState *state,
                              void (^completed)(), void (^failed)(int32_t)) {
    if (!source || !state || !completed || !failed) return -1;
    if (state->phase == ODAsyncClosePhase::Closed ||
        state->phase == ODAsyncClosePhase::Disabling ||
        state->phase == ODAsyncClosePhase::Cancelling) return 0;
    state->phase = ODAsyncClosePhase::Disabling;
    state->error = 0;
    auto disabled = source->SetEnableWithCompletion(false, ^{
        (void)od_async_source_cancel(source, state, completed, failed);
    });
    if (disabled != 0) {
        state->phase = ODAsyncClosePhase::Failed;
        state->error = disabled;
        failed(disabled);
    }
    return disabled;
}
#endif
