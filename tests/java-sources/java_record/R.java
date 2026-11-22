public record R(int x, String s) {
  public R { if (x < 0) throw new IllegalArgumentException("negative"); }
  public String s() { return "S:" + s; }
  public static R of(int x) { return new R(x, "o"); }
}
