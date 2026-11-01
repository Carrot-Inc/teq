package probe;

import java.io.File;
import java.net.URL;
import java.net.URLClassLoader;
import java.util.ArrayList;
import java.util.Arrays;

import sbt.internal.worker1.ForkTestMain;
import sbt.internal.worker1.FrameworkWrapper;
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
 * {@code <class>|<argument>|<suite>,<suite>...}, its suites defined by its first fingerprint (a framework of the old
 * interface wrapped as the worker wraps it, one that cannot be made the probe framework's). With
 * {@code -Dprobe.classpath=<entry>:<entry>...} the run is the worker's virtual layout (`WorkerMain.test`): one class
 * loader over the entries below the worker's, the frameworks and the suites loaded through it. The worker writes its
 * notifications (JSON-RPC lines) and the suites their output on stdout; it never shuts its executor down, so the run
 * ends by exiting.
 */
public final class Pinned {
    public static void main(String[] args) throws Exception {
        boolean parallel = !args[0].equals("1");
        Integer parallelism = args[0].equals("auto") || args[0].equals("1") ? null : Integer.valueOf(args[0]);
        ClassLoader loader = Pinned.class.getClassLoader();
        String classpath = System.getProperty("probe.classpath");
        if (classpath != null) {
            ArrayList<URL> urls = new ArrayList<>();
            for (String entry : classpath.split(File.pathSeparator)) {
                urls.add(new File(entry).toURI().toURL());
            }
            loader = new URLClassLoader(urls.toArray(new URL[0]), ForkTestMain.class.getClassLoader());
        }
        ArrayList<TaskDef> defs = new ArrayList<>();
        ArrayList<TestInfo.TestRunner> runners = new ArrayList<>();
        for (String framework : Arrays.copyOfRange(args, 1, args.length)) {
            String[] fields = framework.split("\\|", -1);
            Fingerprint first;
            try {
                Object raw = loader.loadClass(fields[0]).getDeclaredConstructor().newInstance();
                sbt.testing.Framework f = raw instanceof sbt.testing.Framework n ? n : new FrameworkWrapper((org.scalatools.testing.Framework) raw);
                first = f.fingerprints()[0];
            } catch (ReflectiveOperationException | RuntimeException e) {
                first = new Framework().fingerprints()[0];
            }
            Fingerprint print = new ForkTestMain.SubclassFingerscan((SubclassFingerprint) first);
            for (String suite : fields[2].split(",")) {
                if (!suite.isEmpty()) {
                    defs.add(new TaskDef(suite, print, false, new Selector[] { new SuiteSelector() }));
                }
            }
            runners.add(new TestInfo.TestRunner(new ArrayList<>(Arrays.asList(fields[0])), new ArrayList<>(Arrays.asList(fields[1])), new ArrayList<>()));
        }
        TestInfo info = new TestInfo(true, null, null, false, parallel, parallelism, defs, runners);
        ForkTestMain.main(1L, info, System.out, loader);
        System.out.flush();
        System.exit(0);
    }
}
