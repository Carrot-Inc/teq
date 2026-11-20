package p;
import jdk.internal.misc.Unsafe;
import jdk.internal.misc.Unused;
import q.*;
public class J {
  public static String f(Unsafe u) { return "unsafe"; }
  public static String f(Object o) { return "object"; }
}
