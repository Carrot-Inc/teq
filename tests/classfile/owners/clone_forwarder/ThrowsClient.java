// Calls the forwarder without handling the exception it declares: javac refuses it, as against scalac's class.
public class ThrowsClient {
  public static void main(String[] args) {
    new throwsuse.E().clone();
  }
}
