package probe;

import sbt.testing.*;

/** A test framework whose suite's task does what its argument says: `cause` throws a throwable whose cause cannot be
 * shown (its `toString()` throws), which the worker copies by its message, class and frames alone (ForkTestMain.ForkError),
 * so the fork error is sent; `nulllog` gives its logger a message of null, of which sbt shows nothing, then prints
 * {@code LOGGED}; `serialize` serializes its definition, whose fingerprint is the worker's copy, then prints
 * {@code SERIALIZED}; `tracenull` traces null, which the worker's copy refuses, printing what was thrown; `nulllogger`
 * empties the loggers' array, which the worker's own lines never go through, then reports a success. */
public final class OddReads implements sbt.testing.Framework {
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
    };

    /** A throwable that cannot be shown. */
    static final class Odd extends RuntimeException {
        @Override
        public String toString() {
            throw new IllegalStateException("odd");
        }
    }

    public String name() {
        return "odd-reads";
    }

    public Fingerprint[] fingerprints() {
        return new Fingerprint[] { PRINT };
    }

    public sbt.testing.Runner runner(String[] args, String[] remote, ClassLoader loader) {
        String mode = args.length > 0 ? args[0] : "";
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
                            switch (mode) {
                                case "cause" -> throw new RuntimeException("x", new Odd());
                                case "serialize" -> {
                                    try {
                                        new java.io.ObjectOutputStream(new java.io.ByteArrayOutputStream()).writeObject(def);
                                    } catch (java.io.IOException e) {
                                        throw new java.io.UncheckedIOException(e);
                                    }
                                    System.out.println("SERIALIZED");
                                }
                                case "tracenull" -> {
                                    try {
                                        loggers[0].trace(null);
                                        System.out.println("TRACE TAKEN");
                                    } catch (RuntimeException e) {
                                        System.out.println("TRACE THREW " + e.getClass().getName());
                                    }
                                }
                                case "nulllogger" -> {
                                    loggers[0] = null;
                                    handler.handle(new Event() {
                                        public String fullyQualifiedName() {
                                            return def.fullyQualifiedName();
                                        }

                                        public Fingerprint fingerprint() {
                                            return def.fingerprint();
                                        }

                                        public Selector selector() {
                                            return new TestSelector("case");
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
                                }
                                default -> {
                                    loggers[0].info(null);
                                    System.out.println("LOGGED");
                                }
                            }
                            return new Task[0];
                        }
                    };
                }
                return tasks;
            }
        };
    }
}
