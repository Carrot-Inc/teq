package hidden;

// A public class whose factories answer nested classes of each access: a private one, which no class
// outside this one can name, though it is in this package; one of the package, and a protected one.
public class Nested {
  private static class Secret {
    public final int f() { return 9; }
    public int value = 7;
  }
  static class Packaged {
    public final int f() { return 5; }
  }
  protected static class Guarded {
    public final int f() { return 6; }
  }
  public static Secret secret() { return new Secret(); }
  public static Packaged packaged() { return new Packaged(); }
  public static Guarded guarded() { return new Guarded(); }
}
