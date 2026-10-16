// A cloneable subclass whose `Object.clone`, found by `findVirtual`, clones it: E has no bridge of
// `Object.clone()` to its polymorphic clone. Compiled against scalac's E, run over teq's and scalac's.
import java.lang.invoke.*;

public class Driver extends polyclone.E implements Cloneable {
  public static void main(String[] args) throws Throwable {
    Object result = MethodHandles.lookup().findVirtual(Object.class, "clone", MethodType.methodType(Object.class)).invoke(new Driver());
    System.out.println(result == null ? "null" : result.getClass().getSimpleName());
  }
}
