package hidden;

// Package-private, a library's implementation hidden under a public class: its public members
// are reached through Shown, and a reference naming Base fails with IllegalAccessError outside
// the package. The methods are final, so that javac writes Shown no accessibility bridge for
// them (a public class's bridge would forward to them and hide a reference naming Base).
abstract class Base extends Root implements Defaults {
    public int field = 5;
    public static int staticField = 8;
    public final int method() { return 7; }
    public final int overloaded(int x) { return x + 10; }
    public final int overloaded(String x) { return x.length() + 20; }
    public final int implemented() { return 32; }
    public static int staticMethod() { return 9; }
}
