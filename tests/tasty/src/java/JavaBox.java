package fix.javatemplate;

public class JavaBox<T, U> {
  public T first;

  public JavaBox(T first) {
    this.first = first;
  }

  public <V> V keep(V v) {
    return v;
  }
}
