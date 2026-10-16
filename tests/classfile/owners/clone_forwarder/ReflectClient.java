// Calls the method reflection finds by its name and no parameters: the forwarder, not its bridge.
public class ReflectClient {
    public static void main(String[] args) throws Exception {
        System.out.println(cloneforwarder.E.class.getDeclaredMethod("clone").invoke(new cloneforwarder.E()));
    }
}
