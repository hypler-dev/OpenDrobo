#include "AsyncSourceClose.h"
#include <assert.h>
#include <stdio.h>
#include <initializer_list>

struct SyntheticSource {
    int32_t disable_result = 0, cancel_result = 0;
    unsigned disable_calls = 0, cancel_calls = 0;
    int32_t SetEnableWithCompletion(bool enabled, void (^completion)()) {
        assert(!enabled); ++disable_calls;
        if (!disable_result) completion();
        return disable_result;
    }
    int32_t Cancel(void (^completion)()) {
        ++cancel_calls;
        if (!cancel_result) completion();
        return cancel_result;
    }
};

int main() {
    for (bool disable_failure : {true, false}) {
        SyntheticSource source;
        ODAsyncCloseState state;
        if (disable_failure) source.disable_result = -5;
        else source.cancel_result = -7;
        __block bool owner_retained = true;
        __block unsigned failures = 0, completions = 0;
        auto completed = ^{ owner_retained = false; ++completions; };
        auto failed = ^(int32_t error) { assert(error == -5 || error == -7); ++failures; };
        (void)od_async_source_close(&source, &state, completed, failed);
        assert(state.phase == ODAsyncClosePhase::Failed && state.error != 0);
        assert(owner_retained && failures == 1 && completions == 0);
        assert(source.disable_calls == 1 && source.cancel_calls == (disable_failure ? 0U : 1U));
        source.disable_result = 0; source.cancel_result = 0;
        assert(od_async_source_close(&source, &state, completed, failed) == 0);
        assert(state.phase == ODAsyncClosePhase::Closed && state.error == 0);
        assert(!owner_retained && completions == 1 && failures == 1);
        assert(od_async_source_close(&source, &state, completed, failed) == 0);
        assert(completions == 1 && source.disable_calls == 2);
    }
    SyntheticSource auxiliary;
    ODAsyncCloseState auxiliary_state;
    auxiliary.cancel_result = -7;
    __block bool auxiliary_owner_retained = true;
    __block unsigned auxiliary_errors = 0, auxiliary_completions = 0;
    auto auxiliary_completed = ^{ auxiliary_owner_retained = false; ++auxiliary_completions; };
    auto auxiliary_failed = ^(int32_t error) { assert(error == -7); ++auxiliary_errors; };
    assert(od_async_source_cancel(&auxiliary, &auxiliary_state, auxiliary_completed, auxiliary_failed) == -7);
    assert(auxiliary_owner_retained && auxiliary_errors == 1 && auxiliary_completions == 0);
    assert(auxiliary_state.phase == ODAsyncClosePhase::Failed);
    auxiliary.cancel_result = 0;
    assert(od_async_source_cancel(&auxiliary, &auxiliary_state, auxiliary_completed, auxiliary_failed) == 0);
    assert(!auxiliary_owner_retained && auxiliary_completions == 1 && auxiliary_errors == 1);
    assert(auxiliary.cancel_calls == 2 && auxiliary.disable_calls == 0);
    assert(od_async_source_cancel(&auxiliary, &auxiliary_state, auxiliary_completed, auxiliary_failed) == 0);
    assert(auxiliary_completions == 1 && auxiliary.cancel_calls == 2);
    SyntheticSource pending;
    ODAsyncCloseState state;
    state.phase = ODAsyncClosePhase::Disabling;
    assert(od_async_source_close(&pending, &state, ^{}, ^(int32_t error) { (void)error; assert(false); }) == 0);
    assert(pending.disable_calls == 0 && pending.cancel_calls == 0);
    puts("PASS: failed disable/cancel retains ownership; explicit retry completes once; pending close is not duplicated");
}
