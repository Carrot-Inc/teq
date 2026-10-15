package probe.pin;

import probe.Context;
import probe.Suite;
import sbt.testing.Selector;
import sbt.testing.Status;

/** A task that reports a success of a selector that is not `Serializable`, which the worker refuses. */
public final class BadSelector extends Suite {
    public void run(Context c) {
        c.event(new Selector() {}, Status.Success, null);
    }
}
