package fix;

// Compiled against StubGone, whose class file the fixtures leave out: a signature naming a class
// the class path does not hold (README).
public class StubRef {
  public static String ok() { return "ok"; }
  public static StubGone gone() { return null; }
}
