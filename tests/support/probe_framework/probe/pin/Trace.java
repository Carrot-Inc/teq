package probe.pin;

import probe.Context;
import probe.Suite;

/** A task that has the loggers trace a throwable, then reports a success: the worker's fork error. */
public final class Trace extends Suite {
    public void run(Context c) {
        c.trace(new IllegalStateException("trace-error"));
        c.pass("case");
    }
}
