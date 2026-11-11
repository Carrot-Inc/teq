package fix;

// Compiled against StubGone and StubGoneEx, whose class files the fixtures leave out: a field's type, a type
// parameter's bound and a throws clause naming classes the class path does not hold (README).
public class StubMembers {
  public static StubGone field;
  public static <T extends StubGone> T bounded() { return null; }
  public static void thrower() throws StubGoneEx {}
}
