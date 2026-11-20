import q.Entry;
import java.util.*;
public class J extends AbstractMap<String, String> implements Comparable<Entry> {
  public Set<Map.Entry<String, String>> entrySet() { return Collections.emptySet(); }
  public int compareTo(q.Entry o) { return 0; }
  public static String pick(Entry e) { return "J.pick"; }
}
