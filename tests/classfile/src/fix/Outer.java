package fix;

public class Outer<A> {
    public class Inner<B> {
        public A outerValue;
        public B innerValue;
    }
    public static class Nested implements Runnable {
        public void run() {}
    }
    protected interface Callback<C> { void call(C value); }
    private enum Mode { ON, OFF }

    public Inner<String> make() { return new Inner<String>(); }
    public Outer<A>.Inner<Integer> make2() { return null; }
    public static Nested nested() { return new Nested(); }
    public Runnable anonymous() {
        return new Runnable() { public void run() {} };
    }
}
