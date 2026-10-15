package probe.pin;

import probe.Context;
import probe.Suite;
import sbt.testing.Status;

/** A task that reports an event of a selector the worker cannot write, catching what refusing it would throw, then a
 * success, and returns a task: its events and the task it returns are lost when they are sent. */
public final class OddKind extends Suite {
    public void run(Context c) {
        try {
            c.event(new OddSelector(), Status.Success, null);
            c.out("odd accepted");
        } catch (UnsupportedOperationException e) {
            c.out("odd caught");
        }
        c.pass("case");
        c.nest("probe.pin.OrderChild");
    }
}
