package probe.pin;

import probe.Context;
import probe.Suite;

/** A task that acquires a resource, which its framework's done() releases, then throws. */
public final class Cleanup extends Suite {
    public void run(Context c) throws Exception {
        c.resource();
        throw new IllegalStateException("cleanup-error");
    }
}
