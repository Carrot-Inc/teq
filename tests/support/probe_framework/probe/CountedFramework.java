package probe;
import sbt.testing.*;
/** A test framework that prints each call of its code, its label first (`A made`, `A fingerprints`, `A runner`,
 * `A done`, `A name`, `A ran <suite>`, and its fingerprint's `A isModule`, `A superclassName`,
 * `A requireNoArgConstructor`, `A toString`), so that a run shows the calls sbt's host and worker make of it and their
 * order; its suites are the subclasses of {@link CountedSuite}, each task reporting a success. */
public abstract class CountedFramework implements sbt.testing.Framework {
  private final String label;
  private final SubclassFingerprint print;
  protected CountedFramework(String label) {
    this.label = label;
    System.out.println(label + " made");
    print = new SubclassFingerprint() {
      public boolean isModule() { System.out.println(label + " isModule"); return false; }
      public String superclassName() { System.out.println(label + " superclassName"); return "probe.CountedSuite"; }
      public boolean requireNoArgConstructor() { System.out.println(label + " requireNoArgConstructor"); return true; }
      @Override public String toString() { System.out.println(label + " toString"); return label; }
    };
  }
  public String name() { System.out.println(label + " name"); return label; }
  public Fingerprint[] fingerprints() { System.out.println(label + " fingerprints"); return new Fingerprint[] { print }; }
  public Runner runner(String[] args, String[] remote, ClassLoader loader) {
    System.out.println(label + " runner");
    return new Runner() {
      public String[] args() { return args; }
      public String[] remoteArgs() { return remote; }
      public String done() { System.out.println(label + " done"); return ""; }
      public Task[] tasks(TaskDef[] defs) {
        Task[] tasks = new Task[defs.length];
        for (int i = 0; i < defs.length; i++) {
          TaskDef def = defs[i];
          tasks[i] = new Task() {
            public String[] tags() { return new String[0]; }
            public TaskDef taskDef() { return def; }
            public Task[] execute(EventHandler handler, Logger[] loggers) {
              System.out.println(label + " ran " + def.fullyQualifiedName());
              handler.handle(new Event() {
                public String fullyQualifiedName() { return def.fullyQualifiedName(); }
                public Fingerprint fingerprint() { return def.fingerprint(); }
                public Selector selector() { return new SuiteSelector(); }
                public Status status() { return Status.Success; }
                public OptionalThrowable throwable() { return new OptionalThrowable(); }
                public long duration() { return 1; }
              });
              return new Task[0];
            }
          };
        }
        return tasks;
      }
    };
  }
}
