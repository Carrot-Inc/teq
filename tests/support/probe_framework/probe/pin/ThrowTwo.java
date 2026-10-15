package probe.pin;

import probe.Context;
import probe.Suite;

public final class ThrowTwo extends Suite {
    public void run(Context c) throws Exception {
        c.arrive("clash", 2);
        two();
    }

    private static void two() {
        throw new IllegalArgumentException("two");
    }
}
