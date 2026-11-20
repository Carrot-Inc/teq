package p;
class Outer { }
public class J implements Outer.Absent {
  public Outer.Missing f() { return null; }
  public void g(Outer.Missing a) { }
  public void g(Outer.Other b) { }
  public java.util.Map.Nope k() { return null; }
  public java.nope.Thing n() { return null; }
  public nope.Thing m() { return null; }
}
