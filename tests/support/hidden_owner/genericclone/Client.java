// The forwarders of clone, used raw: javac reads their class files, and reflection reads each result type,
// a type variable only where the class file declares it.
public class Client {
  @SuppressWarnings("rawtypes")
  public static Object generic(genericuse.E e, genericuse.Generic g, genericuse.GenericArray a) {
    return g.clone() == null ? a.clone() : e.clone();
  }

  public static void main(String[] args) throws Exception {
    for (String name : new String[] {"E", "Generic", "GenericArray"}) {
      for (java.lang.reflect.Method m : Class.forName("genericuse." + name).getDeclaredMethods()) {
        if (m.getName().equals("clone") && !m.isBridge()) {
          try {
            System.out.println(m.getGenericReturnType().getTypeName());
          } catch (Throwable t) {
            System.out.println(t);
          }
        }
      }
    }
  }
}
