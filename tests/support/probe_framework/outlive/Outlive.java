package outlive;

import java.util.concurrent.ExecutorService;
import java.util.concurrent.Executors;

/**
 * A library's state that outlives a run in a JVM kept across runs: an executor made at its first use, whose thread
 * takes the context class loader of the thread that made it (as zio's default executor's do), and a count of its uses.
 */
public final class Outlive {
    private static ExecutorService pool;
    private static int uses;

    private static synchronized ExecutorService pool() {
        if (pool == null) {
            pool = Executors.newSingleThreadExecutor(r -> {
                Thread t = new Thread(r, "outlive");
                t.setDaemon(true);
                return t;
            });
        }
        return pool;
    }

    /** Whether the resource is found through the context class loader of the library's thread, the uses so far
     * this one among them, and this JVM's pid. */
    public static synchronized String found(String name) throws Exception {
        uses++;
        boolean found = pool().submit(() -> Thread.currentThread().getContextClassLoader().getResource(name) != null).get();
        return "found " + found + " uses " + uses + " pid " + ProcessHandle.current().pid();
    }
}
