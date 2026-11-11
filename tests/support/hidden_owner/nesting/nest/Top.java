package nest;

// Static member classes ten deep, each its own class file the loader enters from its outer's InnerClasses.
public class Top {
  public static class N1 { public static class N2 { public static class N3 { public static class N4 {
    public static class N5 { public static class N6 { public static class N7 { public static class N8 {
      public static class N9 { public static class N10 {
        public static int value() { return 7; }
      } }
    } } } }
  } } } }
}
