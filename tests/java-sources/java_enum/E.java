public enum E {
  A, B(2), C { public int w() { return 30; } };
  private final int n;
  E() { this(0); }
  E(int n) { this.n = n; }
  public int w() { return n; }
}
