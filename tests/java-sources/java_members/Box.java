import java.util.List;
import java.util.Map;
import java.util.Map.Entry;

public class Box<T extends Comparable<? super T>> {
  public T value;
  public int count;
  public Box(T value) { this.value = value; }
  public static <U extends Comparable<? super U>> Box<U> of(U u) { return new Box<U>(u); }
  public int compareTo(Box<T> other) { return value.compareTo(other.value); }
  public static int sum(int... xs) { int s = 0; for (int x : xs) s += x; return s; }
  public static int size(List<? extends Number> xs) { return xs.size(); }
  @SuppressWarnings("rawtypes")
  public static int rawSize(List xs) { return xs.size(); }
  public static String key(Entry<String, Integer> e) { return e.getKey(); }
  public static Map.Entry<String, Integer> entry(String k, int v) { return new java.util.AbstractMap.SimpleEntry<String, Integer>(k, v); }
  public static int scaled(int count, int by) { return count * by; }
  protected String guarded() { return "guarded"; }
}
