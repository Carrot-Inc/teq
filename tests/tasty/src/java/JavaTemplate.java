package fix.javatemplate;

public class JavaTemplate {
  public int count;

  public JavaTemplate(int count) {
    this.count = count;
  }

  public void reset() {
    count = 0;
  }

  public static int twice(int x) {
    return x * 2;
  }
}
