package probe;
import sbt.testing.*;
/** A test framework whose `fingerprints()` throws a throwable whose `toString()` throws: reporting the run's error
 * throws, which the worker writes as an internal error (ForkTestMain.Run.run, internalError). */
public final class BrokenReport implements sbt.testing.Framework {
  public String name() { return "broken-report"; }
  public Fingerprint[] fingerprints() { throw new Broken(); }
  public Runner runner(String[] a, String[] b, ClassLoader c) { throw new AssertionError(); }
  static final class Broken extends RuntimeException {
    @Override public String toString() { throw new IllegalArgumentException("report-error"); }
  }
}
