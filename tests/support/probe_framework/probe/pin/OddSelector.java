package probe.pin;

import java.io.Serializable;

import sbt.testing.Selector;

/** A selector of none of test-interface's kinds, `Serializable`: the worker takes it, and fails to write it. */
public final class OddSelector extends Selector implements Serializable {
}
