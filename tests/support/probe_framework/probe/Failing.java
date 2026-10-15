package probe;

import sbt.testing.Event;
import sbt.testing.EventHandler;
import sbt.testing.Fingerprint;
import sbt.testing.Logger;
import sbt.testing.OptionalThrowable;
import sbt.testing.Selector;
import sbt.testing.Status;
import sbt.testing.SubclassFingerprint;
import sbt.testing.SuiteSelector;
import sbt.testing.Task;
import sbt.testing.TaskDef;

/**
 * A test framework whose runner fails where its first argument says: `runner` (making its runner), `tasks` (making
 * its tasks) or `done`; its suites are the subclasses of {@link FailingSuite}, each task printing `exec <suite>` and
 * reporting a success.
 */
public final class Failing implements sbt.testing.Framework {
    static final SubclassFingerprint PRINT = new SubclassFingerprint() {
        public boolean isModule() {
            return false;
        }

        public String superclassName() {
            return "probe.FailingSuite";
        }

        public boolean requireNoArgConstructor() {
            return true;
        }
    };

    public String name() {
        return "failing";
    }

    public Fingerprint[] fingerprints() {
        return new Fingerprint[] { PRINT };
    }

    public sbt.testing.Runner runner(String[] args, String[] remoteArgs, ClassLoader loader) {
        String where = args.length > 0 ? args[0] : "";
        if (where.equals("runner")) {
            throw new IllegalStateException("runner-error");
        }
        return new sbt.testing.Runner() {
            public String[] args() {
                return args;
            }

            public String[] remoteArgs() {
                return remoteArgs;
            }

            public Task[] tasks(TaskDef[] defs) {
                if (where.equals("tasks")) {
                    throw new IllegalStateException("tasks-error");
                }
                Task[] tasks = new Task[defs.length];
                for (int i = 0; i < defs.length; i++) {
                    tasks[i] = task(defs[i]);
                }
                return tasks;
            }

            public String done() {
                if (where.equals("done")) {
                    throw new IllegalStateException("done-error");
                }
                return "";
            }
        };
    }

    private static Task task(TaskDef def) {
        return new Task() {
            public String[] tags() {
                return new String[0];
            }

            public TaskDef taskDef() {
                return def;
            }

            public Task[] execute(EventHandler handler, Logger[] loggers) {
                System.out.println("exec " + def.fullyQualifiedName());
                handler.handle(new Event() {
                    public String fullyQualifiedName() {
                        return def.fullyQualifiedName();
                    }

                    public Fingerprint fingerprint() {
                        return PRINT;
                    }

                    public Selector selector() {
                        return new SuiteSelector();
                    }

                    public Status status() {
                        return Status.Success;
                    }

                    public OptionalThrowable throwable() {
                        return new OptionalThrowable();
                    }

                    public long duration() {
                        return 1;
                    }
                });
                return new Task[0];
            }
        };
    }
}
