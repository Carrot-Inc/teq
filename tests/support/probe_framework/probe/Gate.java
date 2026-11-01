package probe;
import sbt.testing.*;
/** A test framework whose fingerprint counts the reads of its module, `module read <n>`, and
 * throws on a seventh: sbt's host reads it six times for one suite named twice (discovery; processOptions' second
 * distinct hashing the suite twice, the second the same definition, equal without a read; filteredFrameworks;
 * forkFingerprint); its suites are the probe framework's, run by the probe framework's runner. */
public final class Gate implements sbt.testing.Framework {
  private int reads;
  private final SubclassFingerprint print = new SubclassFingerprint() {
    public boolean isModule() {
      System.out.println("module read " + ++reads);
      if (reads > 6) throw new IllegalStateException("extra fingerprint read");
      return false;
    }
    public String superclassName() { return "probe.Suite"; }
    public boolean requireNoArgConstructor() { return true; }
    public String toString() { return "gate"; }
  };
  public String name() { return "gate"; }
  public Fingerprint[] fingerprints() { return new Fingerprint[]{print}; }
  public Runner runner(String[] args, String[] remote, ClassLoader loader) {
    return new probe.Framework().runner(args, remote, loader);
  }
}
