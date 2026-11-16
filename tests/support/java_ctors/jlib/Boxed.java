package jlib;

// A generic class with a protected constructor.
public class Boxed<T> {
  public final T x;
  protected Boxed(T x) { this.x = x; }
}
