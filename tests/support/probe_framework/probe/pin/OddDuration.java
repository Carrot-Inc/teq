package probe.pin;

import probe.Context;
import probe.Suite;

/** A task that reports an event of a selector the worker cannot write whose duration cannot be had: the worker's
 * `ForkEvent` reads the duration as the event comes, before the selector is refused when the events are sent, so the
 * task throws. */
public final class OddDuration extends Suite {
    public void run(Context c) {
        c.eventWithoutDuration(new OddSelector());
        c.pass("case");
    }
}
