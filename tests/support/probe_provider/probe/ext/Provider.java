package probe.ext;

import probe.Context;
import probe.Suite;

/**
 * A suite of the probe framework (tests/support/probe_framework) that asks slf4j-api for its logger factory, which
 * slf4j finds among the providers registered where its own class loader sees them (`LoggerFactory`'s
 * `ServiceLoader` over `LoggerFactory.class.getClassLoader()`): it prints the factory's class and passes.
 */
public final class Provider extends Suite {
    public void run(Context c) {
        c.out("factory " + org.slf4j.LoggerFactory.getILoggerFactory().getClass().getName());
        c.pass("provider");
    }
}
