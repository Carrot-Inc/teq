package probe;

import java.util.ArrayList;
import java.util.Arrays;

import sbt.internal.worker1.ForkTestMain;
import sbt.internal.worker1.TestInfo;
import sbt.testing.Fingerprint;
import sbt.testing.Selector;
import sbt.testing.SubclassFingerprint;
import sbt.testing.SuiteSelector;
import sbt.testing.TaskDef;

/**
 * sbt 2.0.8's forked test worker (`ForkTestMain` of the worker jar) over the probe frameworks' suites, as sbt's
 * ForkTests sends them: {@code probe.Pinned <threads> <framework>...}, threads 1 for `testForkedParallel := false`,
 * n for `true` with `testForkedParallelism := Some(n)`, auto for `true` with None, and each framework
 * {@code <class>|<argument>|<suite>,<suite>...}, its suites defined by its fingerprint. The worker writes its
 * notifications (JSON-RPC lines) and the suites their output on stdout; it never shuts its executor down, so the run
 * ends by exiting.
 */
public final class Pinned {
    public static void main(String[] args) throws Exception {
        boolean parallel = !args[0].equals("1");
        Integer parallelism = args[0].equals("auto") || args[0].equals("1") ? null : Integer.valueOf(args[0]);
        ArrayList<TaskDef> defs = new ArrayList<>();
        ArrayList<TestInfo.TestRunner> runners = new ArrayList<>();
        for (String framework : Arrays.copyOfRange(args, 1, args.length)) {
            String[] fields = framework.split("\\|", -1);
            sbt.testing.Framework f = (sbt.testing.Framework) Class.forName(fields[0]).getDeclaredConstructor().newInstance();
            Fingerprint print = new ForkTestMain.SubclassFingerscan((SubclassFingerprint) f.fingerprints()[0]);
            for (String suite : fields[2].split(",")) {
                if (!suite.isEmpty()) {
                    defs.add(new TaskDef(suite, print, false, new Selector[] { new SuiteSelector() }));
                }
            }
            runners.add(new TestInfo.TestRunner(new ArrayList<>(Arrays.asList(fields[0])), new ArrayList<>(Arrays.asList(fields[1])), new ArrayList<>()));
        }
        TestInfo info = new TestInfo(true, null, null, false, parallel, parallelism, defs, runners);
        ForkTestMain.main(1L, info, System.out, Pinned.class.getClassLoader());
        System.out.flush();
        System.exit(0);
    }
}
