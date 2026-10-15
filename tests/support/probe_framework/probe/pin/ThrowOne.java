package probe.pin;

import probe.Context;
import probe.Suite;

/** Two workers: ThrowOne and ThrowTwo throw at once, each its own exception from its own method. */
public final class ThrowOne extends Suite {
    public void run(Context c) throws Exception {
        c.arrive("clash", 2);
        c.pass("before");
        one();
    }

    private static void one() {
        throw new IllegalStateException("one");
    }
}
