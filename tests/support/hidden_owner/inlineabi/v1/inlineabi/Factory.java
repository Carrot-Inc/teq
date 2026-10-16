package inlineabi;

// A package-private class whose public factory answers it, under a public class that inherits f
// (version 1) or declares it (version 2).
class Leaf extends Mid {
}

public class Factory {
  public static Leaf make() { return new Leaf(); }
}
