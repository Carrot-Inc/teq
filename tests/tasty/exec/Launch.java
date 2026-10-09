// tests/tasty/exec/Launch.java: `Launch <class> <method>` runs the static method of the class, with an empty
// argument array where it takes one; what tests/tasty-exec.sh runs a regenerated program by.
import java.lang.reflect.*;
public class Launch {
  public static void main(String[] a) throws Throwable {
    Class<?> c = Class.forName(a[0]);
    for (Method m : c.getMethods()) {
      if (!m.getName().equals(a[1]) || !Modifier.isStatic(m.getModifiers())) continue;
      try {
        if (m.getParameterCount() == 0) m.invoke(null);
        else if (m.getParameterCount() == 1 && m.getParameterTypes()[0] == String[].class) m.invoke(null, (Object) new String[0]);
        else if (m.getParameterCount() == 1 && m.getParameterTypes()[0].getName().equals("scala.collection.immutable.Seq"))
          m.invoke(null, Class.forName("scala.collection.immutable.Nil$").getField("MODULE$").get(null));
        else continue;
      } catch (InvocationTargetException e) { throw e.getCause(); }
      return;
    }
    throw new NoSuchMethodException(a[0] + "." + a[1]);
  }
}
