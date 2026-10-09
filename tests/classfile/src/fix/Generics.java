package fix;

import java.io.IOException;
import java.util.List;
import java.util.Map;
import java.util.function.Function;

public class Generics<T extends Comparable<? super T>> implements Comparable<Generics<T>> {
    public final T value;
    protected List<? extends Number> numbers;
    Map<String, List<? extends Number>> index;
    @SuppressWarnings("rawtypes")
    private List raw;
    public static final int LIMIT = 42;
    public static final String NAME = "generics";
    public static final long BIG = 1L << 40;

    public Generics(T value) { this.value = value; }
    protected Generics(T value, List<? extends Number> numbers) { this(value); this.numbers = numbers; }

    public <U extends Number & Comparable<U>> U max(List<? extends U> xs) { return xs.get(0); }
    public static <E extends Exception> void run(Runnable r) throws E, IOException { r.run(); }
    public <R> R transform(Function<? super T, ? extends R> f) { return f.apply(value); }
    @SafeVarargs
    public static <A> List<A> listOf(A... items) { return List.of(items); }
    public int[] counts(int[][] matrix, T[] items) { return matrix[0]; }
    public byte narrow(short s, float f) { return (byte) (s + f); }
    public void wildcards(List<?> any, Map<? super Integer, ? extends CharSequence> m) {}
    public Map.Entry<String, T> entry() { return null; }
    public int compareTo(Generics<T> other) { return value.compareTo(other.value); }
    @Deprecated
    public void old() {}
    private static synchronized native void internal();
}
