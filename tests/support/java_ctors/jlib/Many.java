package jlib;

// Overloaded constructors of three accesses: which one a creation takes depends on the place.
public class Many {
  public final String s;
  public Many(Object o) { this.s = "public"; }
  protected Many(String o) { this.s = "protected"; }
  Many(Integer o) { this.s = "package"; }
}
