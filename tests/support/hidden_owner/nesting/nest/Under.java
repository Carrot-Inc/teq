package nest;

// An inner class that a subclass's static member class of the same name stands in front of.
public class Under {
  public class Kind { public int n = 6; public Kind() {} }
  public Kind make() { return new Kind(); }
}
