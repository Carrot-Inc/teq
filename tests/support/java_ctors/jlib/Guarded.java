package jlib;

// A public class whose one constructor is protected: its package's and a subclass's super call's.
public class Guarded {
  public final int x;
  protected Guarded(int x) { this.x = x; }
}
