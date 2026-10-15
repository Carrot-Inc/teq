package probe;

/** A suite of the probe framework: its task runs {@link #run} with the run's {@link Context}. */
public abstract class Suite {
    public abstract void run(Context c) throws Exception;
}
