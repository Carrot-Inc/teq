import q.Entry;
import java.util.*;
public class K extends AbstractMap<String, String> {
  public Set<Map.Entry<String, String>> entrySet() { return Collections.emptySet(); }
  public static String pick(Entry e) { return "K.pick"; }
}
