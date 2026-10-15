package probe.pin;

import probe.Context;
import probe.Suite;

public final class OrderB extends Suite {
    public void run(Context c) {
        c.out("exec probe.pin.OrderB");
        c.pass("case");
    }
}
