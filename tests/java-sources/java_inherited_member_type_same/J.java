import java.util.Map.Entry;
import java.util.*;
public class J extends AbstractMap<String, String> {
  public Set<Entry<String, String>> entrySet() { return Set.of(Map.entry("k", "v")); }
  public static String key(Entry<String, String> e) { return e.getKey(); }
}
