package probe;

import org.scalatools.testing.Event;
import org.scalatools.testing.EventHandler;
import org.scalatools.testing.Fingerprint;
import org.scalatools.testing.Logger;
import org.scalatools.testing.Result;
import org.scalatools.testing.Runner2;
import org.scalatools.testing.TestFingerprint;

/**
 * A test framework of sbt's older interface (`org.scalatools.testing`), which sbt's worker runs through its
 * `FrameworkWrapper`: a suite is a subclass of {@link Suite}, which it does not load; its runner logs
 * {@code legacy <suite> <arguments>} and reports one passing test, {@code legacy case}.
 */
public final class Legacy implements org.scalatools.testing.Framework {
    public String name() {
        return "legacy";
    }

    public Fingerprint[] tests() {
        return new Fingerprint[] { new TestFingerprint() {
            public boolean isModule() {
                return false;
            }

            public String superClassName() {
                return "probe.Suite";
            }
        } };
    }

    public org.scalatools.testing.Runner testRunner(ClassLoader loader, Logger[] loggers) {
        return new Runner2() {
            public void run(String testClassName, Fingerprint fingerprint, EventHandler handler, String[] args) {
                loggers[0].info("legacy " + testClassName + " " + String.join(",", args));
                handler.handle(new Event() {
                    public String testName() {
                        return "legacy case";
                    }

                    public String description() {
                        return "";
                    }

                    public Result result() {
                        return Result.Success;
                    }

                    public Throwable error() {
                        return null;
                    }
                });
            }
        };
    }
}
