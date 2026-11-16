package jlib;
// A class whose one constructor is protected and takes a variable number of arguments: created with none written
// by an anonymous subclass of an empty body, it is that subclass's super call.
public class Spread {
  public final int n;
  protected Spread(String... xs) { n = xs.length; }
}
