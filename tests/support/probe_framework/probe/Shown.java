package probe;

import sbt.testing.*;

/** A test framework whose fingerprint cannot be hashed (its `hashCode()` throws, its `toString()`, which sbt's discovery
 * shows, its own), the suites' definitions carrying the
 * copies sbt's host makes of it (ForkTests.forkFingerprint, ForkTestMain.SubclassFingerscan), and whose second
 * fingerprint, of no suite, cannot say whether it requires a constructor without arguments, which sbt reads only of a
 * suite's fingerprint: its suite runs, printing {@code ROOT RAN}. */
public final class Shown implements sbt.testing.Framework {
    static final SubclassFingerprint PRINT = new SubclassFingerprint() {
        public boolean isModule() {
            return false;
        }

        public String superclassName() {
            return "probe.Suite";
        }

        public boolean requireNoArgConstructor() {
            return true;
        }

        @Override
        public String toString() {
            return "Shown.PRINT";
        }

        @Override
        public int hashCode() {
            throw new IllegalStateException("hash-error");
        }
    };

    static final SubclassFingerprint UNREQUIRED = new SubclassFingerprint() {
        public boolean isModule() {
            return false;
        }

        public String superclassName() {
            return "probe.Never";
        }

        public boolean requireNoArgConstructor() {
            throw new IllegalStateException("required-error");
        }
    };

    public String name() {
        return "shown";
    }

    public Fingerprint[] fingerprints() {
        return new Fingerprint[] { PRINT, UNREQUIRED };
    }

    public sbt.testing.Runner runner(String[] args, String[] remote, ClassLoader loader) {
        return new sbt.testing.Runner() {
            public String[] args() {
                return args;
            }

            public String[] remoteArgs() {
                return remote;
            }

            public String done() {
                return "";
            }

            public Task[] tasks(TaskDef[] defs) {
                Task[] tasks = new Task[defs.length];
                for (int i = 0; i < defs.length; i++) {
                    TaskDef def = defs[i];
                    tasks[i] = new Task() {
                        public String[] tags() {
                            return new String[0];
                        }

                        public TaskDef taskDef() {
                            return def;
                        }

                        public Task[] execute(EventHandler handler, Logger[] loggers) {
                            System.out.println("ROOT RAN");
                            return new Task[0];
                        }
                    };
                }
                return tasks;
            }
        };
    }
}
