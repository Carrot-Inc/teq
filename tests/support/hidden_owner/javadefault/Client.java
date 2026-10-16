// The annotation of E's forwarder of clone, read by reflection.
public class Client {
  public static void main(String[] args) throws Exception {
    System.out.println(jannuse.E.class.getDeclaredMethod("clone").getAnnotation(jann.Tag.class));
  }
}
