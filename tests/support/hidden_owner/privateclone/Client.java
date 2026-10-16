// Calls E's clone, which is `Object`'s protected one: javac refuses it, as against scalac's class.
public class Client {
  public static void main(String[] args) {
    System.out.println(new privatecloneuse.E().clone());
  }
}
