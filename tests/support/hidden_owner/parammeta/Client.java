// The parameters' annotations of E's clone(String) and equals(Object), read by reflection.
import java.util.Arrays;

public class Client {
  public static void main(String[] args) throws Exception {
    System.out.println(Arrays.deepToString(pmuse.E.class.getMethod("clone", String.class).getParameterAnnotations()));
    System.out.println(Arrays.deepToString(pmuse.E.class.getMethod("equals", Object.class).getParameterAnnotations()));
  }
}
