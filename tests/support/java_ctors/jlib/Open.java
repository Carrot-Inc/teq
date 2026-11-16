package jlib;
// A generic class with a public constructor that takes nothing beside a protected one: named bare with an empty
// body, the creation is the public one's.
public class Open<T> {
  public Open() {}
  protected Open(int x) {}
  public int v() { return 13; }
}
