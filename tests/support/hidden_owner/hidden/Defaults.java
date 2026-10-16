package hidden;

public interface Defaults {
    default int defaultOnly() { return 31; }
    default int implemented() { return 30; }
}
