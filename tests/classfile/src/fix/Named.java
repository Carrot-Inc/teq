package fix;

import java.util.List;

public abstract class Named<K, V extends List<K>> {
    public abstract V lookup(K key, int limit);
    public static String join(String separator, String... parts) { return String.join(separator, parts); }
    protected Named(K first) {}
}
