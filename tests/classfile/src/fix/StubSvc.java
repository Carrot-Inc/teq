package fix;

// Compiled against javax.inject.Provider and scala.xml.Elem, whose class files the fixtures leave out: signatures
// naming classes of a JDK root and of scala that neither the JDK nor scala-library holds (README).
public class StubSvc {
  public static String ok() { return "ok"; }
  public static javax.inject.Provider<String> provider() { return null; }
  public static scala.xml.Elem elem() { return null; }
}
