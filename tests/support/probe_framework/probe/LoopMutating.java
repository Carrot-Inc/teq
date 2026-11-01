package probe;
import sbt.testing.*;
import java.util.concurrent.atomic.AtomicInteger;
/** Its first fingerprint's isModule() replaces the array's second entry (B by C) on its first call: a loop that
 * reads the array live meets C, one over a copy taken before meets B. */
public final class LoopMutating implements sbt.testing.Framework {
  static final AtomicInteger made = new AtomicInteger();
  final int id = made.incrementAndGet();
  boolean replaced;
  private Fingerprint fp(String label, String superclass, boolean mutate) {
    return new SubclassFingerprint() {
      public boolean isModule() {
        System.out.println("instance " + id + " " + label + " isModule");
        if (mutate && !replaced) { replaced = true; prints[1] = fp("C", "probe.Never", false); }
        return false;
      }
      public String superclassName() { return superclass; }
      public boolean requireNoArgConstructor() { return true; }
      public String toString() { return label; }
    };
  }
  final Fingerprint[] prints = { fp("A", "probe.Suite", true), fp("B", "probe.Never", false) };
  public String name() { return "loop"; }
  public Fingerprint[] fingerprints() { return prints; }
  public Runner runner(String[] args, String[] remote, ClassLoader loader) {
    return new probe.Framework().runner(args, remote, loader);
  }
}
