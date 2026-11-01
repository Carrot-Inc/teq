package probe;
import sbt.testing.*;
import java.util.Arrays;
/** A framework whose task, as `mutate`, replaces its TaskDef's selectors array's first entry, and otherwise prints it. */
abstract class SelFramework implements sbt.testing.Framework {
  static final SubclassFingerprint PRINT = new SubclassFingerprint() {
    public boolean isModule() { return false; }
    public String superclassName() { return "probe.Suite"; }
    public boolean requireNoArgConstructor() { return true; }
  };
  abstract String label();
  abstract boolean mutate();
  public String name() { return label(); }
  public Fingerprint[] fingerprints() { return new Fingerprint[]{PRINT}; }
  public Runner runner(String[] args, String[] remote, ClassLoader loader) {
    return new Runner() {
      public Task[] tasks(TaskDef[] defs) {
        Task[] out = new Task[defs.length];
        for (int i = 0; i < defs.length; i++) {
          TaskDef def = defs[i];
          out[i] = new Task() {
            public String[] tags() { return new String[0]; }
            public TaskDef taskDef() { return def; }
            public Task[] execute(EventHandler h, Logger[] l) {
              if (mutate()) def.selectors()[0] = new TestSelector("mutated-by-" + label());
              System.out.println(label() + " sees " + Arrays.toString(def.selectors()));
              return new Task[0];
            }
          };
        }
        return out;
      }
      public String done() { return ""; }
      public String[] remoteArgs() { return remote; }
      public String[] args() { return args; }
    };
  }
}
